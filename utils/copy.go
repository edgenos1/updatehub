package utils

import (
	"errors"
	"io"
	"time"
)

const bufferSize = 32 * 1024

// Copy copies from rd to wr until EOF or timeout is reached on rd or it was cancelled
func Copy(wr io.Writer, rd io.Reader, timeout time.Duration, cancel <-chan bool) (bool, error) {
	len := make(chan int)
	buf := make([]byte, bufferSize)
	readErrChan := make(chan error)

Loop:
	for {
		go func() {
			n, err := rd.Read(buf)
			if n == 0 && err != nil {
				if err != io.EOF {
					readErrChan <- err
				}
				close(len)
			} else {
				len <- n
			}
		}()

		select {
		case err, ok := <-readErrChan:
			if ok {
				close(readErrChan)
				return false, err
			}
		case _, ok := <-cancel:
			if ok {
				return true, nil
			}
		case <-time.After(timeout):
			return false, errors.New("timeout")
		case n, ok := <-len:
			if !ok {
				break Loop
			}

			_, err := wr.Write(buf[0:n])
			if err != nil {
				return false, err
			}
		}
	}

	return false, nil
}