// `echo` (design/async.md §6): a TCP echo server on 0.0.0.0:<port>, one
// goroutine per connection, which reads up to 64 bytes and writes them back
// until the end of the stream. GOMAXPROCS=1 runs it on one thread.
//
//	go run ./echo <port>
package main

import (
	"net"
	"os"
)

func main() {
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
			buffer := make([]byte, 64)
			for {
				read, err := conn.Read(buffer)
				if err != nil {
					return
				}
				if _, err := conn.Write(buffer[:read]); err != nil {
					return
				}
			}
		}()
	}
}
