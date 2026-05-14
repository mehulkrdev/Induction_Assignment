package logger

import (
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
	"sync"
	"time"
)

var (
	logFile *os.File
	logger  *log.Logger
	mu      sync.Mutex
	logDir  string
)

func InitLogger(componentName string) {
	mu.Lock()
	defer mu.Unlock()

	// Container-safe log root directory
	logRootDir := "/Assignment/Logs"

	// Create timestamped directory
	currentTime := time.Now().Format("02-01-2006_15-04")
	logDir = filepath.Join(logRootDir, currentTime)

	if err := os.MkdirAll(logDir, 0755); err != nil {
		log.Fatalf("Failed to create log directory: %v", err)
	}

	// Create log file
	logFilePath := filepath.Join(logDir, fmt.Sprintf("%s.log", componentName))

	var err error
	logFile, err = os.OpenFile(
		logFilePath,
		os.O_APPEND|os.O_CREATE|os.O_WRONLY,
		0644,
	)

	if err != nil {
		log.Fatalf("Failed to open log file: %v", err)
	}

	// Write logs to both file and stdout
	logger = log.New(
		io.MultiWriter(logFile, os.Stdout),
		"",
		log.Ldate|log.Ltime|log.Lshortfile,
	)

	// IMPORTANT:
	// Do NOT call Logf() here because mutex is already locked
	logger.Printf("Logger initialized for component: %s", componentName)
}

func Logf(format string, v ...interface{}) {
	mu.Lock()
	defer mu.Unlock()

	if logger == nil {
		log.Printf("Logger not initialized: "+format, v...)
		return
	}

	logger.Printf(format, v...)
}

func CleanupLogDir() {
	mu.Lock()
	defer mu.Unlock()

	if logFile != nil {
		if err := logFile.Close(); err != nil {
			log.Printf("Failed to close log file: %v", err)
		}
	}
}