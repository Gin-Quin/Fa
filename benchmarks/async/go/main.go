// Spawns n goroutines (default 100 000) that each yield once to the
// scheduler then store 2 * index, waits for them all, and prints the sum
// (n * (n - 1)) and the time taken. GOMAXPROCS=1 runs them on one thread.
//
//	go run . [n]
package main

import (
	"fmt"
	"os"
	"runtime"
	"strconv"
	"sync"
	"time"
)

func main() {
	n := 100_000
	if len(os.Args) > 1 {
		parsed, err := strconv.Atoi(os.Args[1])
		if err != nil {
			panic(err)
		}
		n = parsed
	}
	start := time.Now()
	results := make([]int64, n)
	var group sync.WaitGroup
	group.Add(n)
	for index := 0; index < n; index++ {
		go func() {
			runtime.Gosched()
			results[index] = int64(index) * 2
			group.Done()
		}()
	}
	group.Wait()
	var sum int64
	for _, result := range results {
		sum += result
	}
	elapsed := time.Since(start)
	fmt.Printf("checksum %d\n", sum)
	fmt.Printf("go GOMAXPROCS=%d: %d goroutines in %.2f ms (%.0f ns per goroutine)\n",
		runtime.GOMAXPROCS(0), n, float64(elapsed.Nanoseconds())/1e6,
		float64(elapsed.Nanoseconds())/float64(n))
}
