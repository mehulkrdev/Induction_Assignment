package main

import (
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestEnrollmentEndpoint(t *testing.T) {
	req, err := http.NewRequest("GET", "/enroll", nil)
	if err != nil {
		t.Fatal(err)
	}

	rr := httptest.NewRecorder()
	handler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprintf(w, "Enrollment endpoint hit!")
	})

	handler.ServeHTTP(rr, req)

	if status := rr.Code; status != http.StatusOK {
		t.Errorf("handler returned wrong status code: got %v want %v", status, http.StatusOK)
	}

	expected := "Enrollment endpoint hit!"
	if rr.Body.String() != expected {
		t.Errorf("handler returned unexpected body: got %v want %v", rr.Body.String(), expected)
	}
}
