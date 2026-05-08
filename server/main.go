package main

import (
	"fmt"
	"net/http"
)

// EnrollmentHandler is the exported handler for the /enroll endpoint
func EnrollmentHandler(w http.ResponseWriter, r *http.Request) {
	// Not implemented yet - RED phase
	// This will make tests fail as expected (we expect 200, 400, 401 but will get 501)
	http.Error(w, "Not implemented", http.StatusNotImplemented)
}

func main() {
	http.HandleFunc("/enroll", EnrollmentHandler)

	fmt.Println("Server listening on :8443")
	err := http.ListenAndServe(":8443", nil)
	if err != nil {
		panic(err)
	}
}
