// `starve` (design/async.md §6): 1000 goroutines spin forever on their own
// always-ready channel of capacity 1 (send, then receive), one goroutine
// sleeps 1 ms <ticks> times (default 3000) and records how late each wake
// is, and an accept loop on 0.0.0.0:<port> writes the byte `x` to each
// connection then closes it. Once the ticks are done, prints the lateness
// percentiles and exits. GOMAXPROCS=1 runs it on one thread.
//
//	go run ./starve <port> [ticks]
package main

import (
	"fmt"
	"net"
	"os"
	"slices"
	"strconv"
	"time"
)

func percentile(sorted []time.Duration, p float64) int64 {
	rank := max(0, min(int(p*float64(len(sorted))+0.999999)-1, len(sorted)-1))
	return sorted[rank].Microseconds()
}

func main() {
	ticks := 3000
	if len(os.Args) > 2 {
		parsed, err := strconv.Atoi(os.Args[2])
		if err != nil {
			panic(err)
		}
		ticks = parsed
	}
	listener, err := net.Listen("tcp", ":"+os.Args[1])
	if err != nil {
		panic(err)
	}
	for range 1000 {
		go func() {
			channel := make(chan int, 1)
			value := 0
			for {
				channel <- value
				value = <-channel + 1
			}
		}()
	}
	go func() {
		for {
			conn, err := listener.Accept()
			if err != nil {
				continue
			}
			conn.Write([]byte{'x'})
			conn.Close()
		}
	}()
	lateness := make([]time.Duration, 0, ticks)
	for range ticks {
		start := time.Now()
		time.Sleep(time.Millisecond)
		lateness = append(lateness, time.Since(start)-time.Millisecond)
	}
	slices.Sort(lateness)
	fmt.Printf("lateness_p50_us=%d lateness_p99_us=%d lateness_max_us=%d\n",
		percentile(lateness, 0.5), percentile(lateness, 0.99),
		lateness[len(lateness)-1].Microseconds())
	os.Exit(0)
}
