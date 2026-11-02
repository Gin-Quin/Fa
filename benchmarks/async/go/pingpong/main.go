// `ping-pong` (design/async.md §6): two goroutines exchange n messages
// (default 10 000 000, n / 2 round trips) over two channels of capacity 1,
// and print the last ball and the time taken. GOMAXPROCS=1 runs them on
// one thread.
//
//	go run ./pingpong [n]
package main

import (
	"fmt"
	"os"
	"runtime"
	"strconv"
	"time"
)

func main() {
	n := 10_000_000
	if len(os.Args) > 1 {
		parsed, err := strconv.Atoi(os.Args[1])
		if err != nil {
			panic(err)
		}
		n = parsed
	}
	start := time.Now()
	pings := make(chan int64, 1)
	pongs := make(chan int64, 1)
	done := make(chan struct{})
	go func() {
		for ball := range pings {
			pongs <- ball + 1
		}
		close(done)
	}()
	var ball int64
	for round := 0; round < n/2; round++ {
		pings <- ball
		ball = <-pongs
	}
	close(pings)
	<-done
	elapsed := time.Since(start)
	fmt.Printf("ball %d\n", ball)
	fmt.Printf("go GOMAXPROCS=%d: %d messages in %.2f ms (%.1f ns per message)\n",
		runtime.GOMAXPROCS(0), n, float64(elapsed.Nanoseconds())/1e6,
		float64(elapsed.Nanoseconds())/float64(n))
}
