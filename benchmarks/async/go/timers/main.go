// `timers` (design/async.md §6): n goroutines (default 1 000 000), the i-th
// sleeping (i * 7919) % 1001 ms, plus one goroutine sleeping 1 ms in a loop
// until they all ended and recording how late each wake is; prints the
// total time and the lateness percentiles. GOMAXPROCS=1 runs it on one
// thread.
//
//	go run ./timers [n]
package main

import (
	"fmt"
	"os"
	"slices"
	"strconv"
	"sync"
	"sync/atomic"
	"time"
)

func percentile(sorted []time.Duration, p float64) int64 {
	rank := max(0, min(int(p*float64(len(sorted))+0.999999)-1, len(sorted)-1))
	return sorted[rank].Microseconds()
}

func main() {
	n := 1_000_000
	if len(os.Args) > 1 {
		parsed, err := strconv.Atoi(os.Args[1])
		if err != nil {
			panic(err)
		}
		n = parsed
	}
	start := time.Now()
	var done atomic.Bool
	lateness := []time.Duration{}
	ticked := make(chan struct{})
	go func() {
		for !done.Load() {
			before := time.Now()
			time.Sleep(time.Millisecond)
			lateness = append(lateness, time.Since(before)-time.Millisecond)
		}
		close(ticked)
	}()
	var group sync.WaitGroup
	group.Add(n)
	for index := range n {
		go func() {
			time.Sleep(time.Duration((index*7919)%1001) * time.Millisecond)
			group.Done()
		}()
	}
	group.Wait()
	total := time.Since(start)
	done.Store(true)
	<-ticked
	slices.Sort(lateness)
	fmt.Printf("total_ms=%d lateness_p99_us=%d lateness_max_us=%d\n",
		total.Milliseconds(), percentile(lateness, 0.99),
		lateness[len(lateness)-1].Microseconds())
}
