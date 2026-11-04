// The load generator of the B2 async benchmarks (design/async.md §6). Each
// subcommand prints one machine-readable line `key=value …` on stdout (and a
// human summary on stderr); SIGINT or SIGTERM ends the measurement early and
// still prints it.
//
//	load echo -addr 127.0.0.1:7878 -duration 20s -connections 10000 -size 64
//	    N connections, each loops: write `size` bytes, read `size` bytes back.
//	    The window starts once every connection is open.
//	    → requests rps p50_us p99_us p999_us errors
//	load fairness -addr … -duration … -short 64 -long 4 -size 64
//	    The echo protocol; the first byte of a request is `s` (short: echoed
//	    at once) or `l` (long: 20 ms of CPU, then echoed).
//	    → short_requests long_requests p50_us p99_us p999_us (short ones)
//	      long_p50_us errors
//	load flood -addr … -duration … -connections 5000 -rate 0
//	    Opens connections as fast as possible (or `rate` per second), at most
//	    N at once (the oldest open one is closed to make room), and measures
//	    the time from the dial to the first byte, which the server writes
//	    right after accepting.
//	    → connections ttfb_p50_us ttfb_p99_us ttfb_max_us old_p99_us
//	      unanswered errors (old: the 10% of connections dialed first)
package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"net"
	"os"
	"os/signal"
	"slices"
	"sync"
	"sync/atomic"
	"syscall"
	"time"
)

// stopped is closed on SIGINT or SIGTERM.
var stopped = make(chan struct{})

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: load echo|fairness|flood [flags]")
		os.Exit(2)
	}
	signals := make(chan os.Signal, 1)
	signal.Notify(signals, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-signals
		close(stopped)
	}()
	command, arguments := os.Args[1], os.Args[2:]
	flags := flag.NewFlagSet(command, flag.ExitOnError)
	addr := flags.String("addr", "127.0.0.1:7878", "server address")
	duration := flags.Duration("duration", 10*time.Second, "measurement window")
	switch command {
	case "echo":
		connections := flags.Int("connections", 10000, "connections")
		size := flags.Int("size", 64, "request size in bytes")
		flags.Parse(arguments)
		echo(*addr, *duration, *connections, *size)
	case "fairness":
		short := flags.Int("short", 64, "short clients")
		long := flags.Int("long", 4, "long clients")
		size := flags.Int("size", 64, "request size in bytes")
		flags.Parse(arguments)
		fairness(*addr, *duration, *short, *long, *size)
	case "flood":
		connections := flags.Int("connections", 5000, "connections open at once")
		rate := flags.Int("rate", 0, "connections per second (0: as fast as possible)")
		flags.Parse(arguments)
		flood(*addr, *duration, *connections, *rate)
	default:
		fmt.Fprintf(os.Stderr, "unknown subcommand %q\n", command)
		os.Exit(2)
	}
}

// percentile of sorted samples (nearest rank), in microseconds.
func percentile(sorted []time.Duration, p float64) int64 {
	if len(sorted) == 0 {
		return 0
	}
	rank := int(p*float64(len(sorted))+0.999999) - 1
	rank = max(0, min(rank, len(sorted)-1))
	return sorted[rank].Microseconds()
}

func isStopped() bool {
	select {
	case <-stopped:
		return true
	default:
		return false
	}
}

// dial retries with an exponential backoff (10 ms to 1 s, 30 attempts).
func dial(addr string) (net.Conn, error) {
	backoff := 10 * time.Millisecond
	var err error
	for attempt := 0; attempt < 30 && !isStopped(); attempt++ {
		var conn net.Conn
		conn, err = net.DialTimeout("tcp", addr, 5*time.Second)
		if err == nil {
			return conn, nil
		}
		time.Sleep(backoff)
		backoff = min(2*backoff, time.Second)
	}
	return nil, err
}

// A client loops on requests of one kind and keeps its latencies.
type client struct {
	kind    byte
	samples []time.Duration
	errors  int64
}

// runClients opens every connection (at most 256 dials at once), then starts
// the window, and returns its real length.
func runClients(addr string, duration time.Duration, clients []*client, size int) time.Duration {
	ready := make(chan struct{})
	var end atomic.Int64
	var opened, done sync.WaitGroup
	dials := make(chan struct{}, 256)
	opened.Add(len(clients))
	done.Add(len(clients))
	for _, c := range clients {
		go func() {
			defer done.Done()
			dials <- struct{}{}
			conn, err := dial(addr)
			<-dials
			opened.Done()
			<-ready
			request := make([]byte, size)
			for i := range request {
				request[i] = c.kind
			}
			response := make([]byte, size)
			for {
				if err != nil {
					c.errors++
					if conn != nil {
						conn.Close()
					}
					if time.Now().UnixNano() >= end.Load() || isStopped() {
						return
					}
					conn, err = dial(addr)
					continue
				}
				deadline := end.Load()
				if time.Now().UnixNano() >= deadline || isStopped() {
					conn.Close()
					return
				}
				conn.SetDeadline(time.Unix(0, deadline).Add(5 * time.Second))
				start := time.Now()
				if _, err = conn.Write(request); err != nil {
					continue
				}
				if _, err = io.ReadFull(conn, response); err != nil {
					continue
				}
				finished := time.Now()
				if finished.UnixNano() < end.Load() {
					c.samples = append(c.samples, finished.Sub(start))
				}
			}
		}()
	}
	opened.Wait()
	start := time.Now()
	end.Store(start.Add(duration).UnixNano())
	fmt.Fprintf(os.Stderr, "%d connections open, measuring for %v\n", len(clients), duration)
	close(ready)
	go func() {
		<-stopped
		end.Store(time.Now().UnixNano())
	}()
	done.Wait()
	return min(time.Since(start), time.Duration(end.Load()-start.UnixNano()))
}

func merge(clients []*client) ([]time.Duration, int64) {
	var all []time.Duration
	var errors int64
	for _, c := range clients {
		all = append(all, c.samples...)
		errors += c.errors
	}
	slices.Sort(all)
	return all, errors
}

func echo(addr string, duration time.Duration, connections, size int) {
	clients := make([]*client, connections)
	for i := range clients {
		clients[i] = &client{kind: 'e'}
	}
	window := runClients(addr, duration, clients, size)
	samples, errors := merge(clients)
	rps := float64(len(samples)) / window.Seconds()
	fmt.Fprintf(os.Stderr, "%d requests in %v (%.0f/s), %d errors\n", len(samples), window, rps, errors)
	fmt.Printf("requests=%d rps=%.0f p50_us=%d p99_us=%d p999_us=%d errors=%d\n",
		len(samples), rps, percentile(samples, 0.5), percentile(samples, 0.99),
		percentile(samples, 0.999), errors)
}

func fairness(addr string, duration time.Duration, short, long, size int) {
	shortClients := make([]*client, short)
	longClients := make([]*client, long)
	var clients []*client
	for i := range shortClients {
		shortClients[i] = &client{kind: 's'}
		clients = append(clients, shortClients[i])
	}
	for i := range longClients {
		longClients[i] = &client{kind: 'l'}
		clients = append(clients, longClients[i])
	}
	runClients(addr, duration, clients, size)
	shortSamples, shortErrors := merge(shortClients)
	longSamples, longErrors := merge(longClients)
	fmt.Fprintf(os.Stderr, "%d short, %d long requests\n", len(shortSamples), len(longSamples))
	fmt.Printf("short_requests=%d long_requests=%d p50_us=%d p99_us=%d p999_us=%d long_p50_us=%d errors=%d\n",
		len(shortSamples), len(longSamples), percentile(shortSamples, 0.5),
		percentile(shortSamples, 0.99), percentile(shortSamples, 0.999),
		percentile(longSamples, 0.5), shortErrors+longErrors)
}

type attempt struct {
	dialed time.Time
	ttfb   time.Duration
}

func flood(addr string, duration time.Duration, limit, rate int) {
	var mu sync.Mutex
	cond := sync.NewCond(&mu)
	var open []net.Conn
	dialing := 0
	reading := map[net.Conn]struct{}{}
	launched := 0
	var attempts []attempt
	var errorCount, unanswered int64
	var running sync.WaitGroup
	start := time.Now()
	end := start.Add(duration)
	finished := func() bool { return time.Now().After(end) || isStopped() }
	readDeadline := end // moved to the real end, under mu
	dialContext, cancel := context.WithDeadline(context.Background(), end.Add(time.Second))
	defer cancel()
	go func() {
		<-stopped
		cancel()
	}()
	var dialer net.Dialer
	// wakes the launcher so that it notices the end
	go func() {
		for !finished() {
			time.Sleep(10 * time.Millisecond)
			cond.Broadcast()
		}
		cond.Broadcast()
	}()
	for !finished() {
		if rate > 0 {
			next := start.Add(time.Duration(launched) * time.Second / time.Duration(rate))
			if wait := time.Until(next); wait > 0 {
				time.Sleep(wait)
				continue
			}
		}
		mu.Lock()
		for dialing+len(reading)+len(open) >= limit && !finished() {
			if len(open) > 0 {
				open[0].Close()
				open = open[1:]
			} else {
				cond.Wait()
			}
		}
		mu.Unlock()
		if finished() {
			break
		}
		launched++
		running.Add(1)
		mu.Lock()
		dialing++
		mu.Unlock()
		go func() {
			defer running.Done()
			dialed := time.Now()
			conn, err := dialer.DialContext(dialContext, "tcp", addr)
			mu.Lock()
			dialing--
			if err != nil {
				if !finished() {
					if errorCount++; errorCount <= 3 {
						fmt.Fprintln(os.Stderr, "dial:", err)
					}
				}
				cond.Signal()
				mu.Unlock()
				return
			}
			conn.(*net.TCPConn).SetLinger(0) // no TIME_WAIT: ports stay free
			reading[conn] = struct{}{}
			conn.SetReadDeadline(readDeadline)
			mu.Unlock()
			var first [1]byte
			_, err = conn.Read(first[:])
			ttfb := time.Since(dialed)
			mu.Lock()
			defer mu.Unlock()
			delete(reading, conn)
			cond.Signal()
			if err != nil {
				var timeout net.Error
				if errors.As(err, &timeout) && timeout.Timeout() {
					unanswered++
				} else if errorCount++; errorCount <= 3 {
					fmt.Fprintln(os.Stderr, "read:", err)
				}
				conn.Close()
				return
			}
			attempts = append(attempts, attempt{dialed, ttfb})
			open = append(open, conn)
		}()
	}
	// the end: unblock the reads still waiting
	mu.Lock()
	readDeadline = time.Now()
	for conn := range reading {
		conn.SetReadDeadline(time.Now())
	}
	mu.Unlock()
	running.Wait()
	for _, conn := range open {
		conn.Close()
	}
	slices.SortFunc(attempts, func(a, b attempt) int { return a.dialed.Compare(b.dialed) })
	oldCount := max(1, len(attempts)/10)
	old := make([]time.Duration, 0, oldCount)
	for _, a := range attempts[:min(oldCount, len(attempts))] {
		old = append(old, a.ttfb)
	}
	slices.Sort(old)
	all := make([]time.Duration, len(attempts))
	for i, a := range attempts {
		all[i] = a.ttfb
	}
	slices.Sort(all)
	maximum := int64(0)
	if len(all) > 0 {
		maximum = all[len(all)-1].Microseconds()
	}
	fmt.Fprintf(os.Stderr, "%d connections answered, %d unanswered, %d errors in %v\n",
		len(all), unanswered, errorCount, time.Since(start))
	fmt.Printf("connections=%d ttfb_p50_us=%d ttfb_p99_us=%d ttfb_max_us=%d old_p99_us=%d unanswered=%d errors=%d\n",
		len(all), percentile(all, 0.5), percentile(all, 0.99), maximum,
		percentile(old, 0.99), unanswered, errorCount)
}
