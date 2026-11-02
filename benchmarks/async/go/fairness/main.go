// `fairness` (design/async.md §6): a TCP server on 0.0.0.0:<port> reading
// 64-byte requests; one starting with `l` first runs 20 ms of CPU (a busy
// loop until the monotonic clock says 20 ms passed), calling
// runtime.Gosched() every 1 ms in the `yield` variant and never in `spin`;
// then every request is written back. GOMAXPROCS=1 runs it on one thread.
//
//	go run ./fairness <port> <yield|spin>
package main

import (
	"io"
	"net"
	"os"
	"runtime"
	"time"
)

func work(yield bool) {
	start := time.Now()
	lastYield := start
	for time.Since(start) < 20*time.Millisecond {
		if yield && time.Since(lastYield) >= time.Millisecond {
			runtime.Gosched()
			lastYield = time.Now()
		}
	}
}

func main() {
	yield := os.Args[2] == "yield"
	if !yield && os.Args[2] != "spin" {
		panic("the variant is `yield` or `spin`")
	}
	listener, err := net.Listen("tcp", ":"+os.Args[1])
	if err != nil {
		panic(err)
	}
	for {
		conn, err := listener.Accept()
		if err != nil {
			continue
		}
		go func() {
			defer conn.Close()
			request := make([]byte, 64)
			for {
				if _, err := io.ReadFull(conn, request); err != nil {
					return
				}
				if request[0] == 'l' {
					work(yield)
				}
				if _, err := conn.Write(request); err != nil {
					return
				}
			}
		}()
	}
}
