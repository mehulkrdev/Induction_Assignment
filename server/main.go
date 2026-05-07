package main

import (
	"fmt"
	"net/http"
)

func main() {
	http.HandleFunc("/enroll", func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprintf(w, "Enrollment endpoint hit!")
	})

	fmt.Println("Server listening on :8443")
	err := http.ListenAndServe(":8443", nil)
		if err != nil {
			panic(err)
		}
}
