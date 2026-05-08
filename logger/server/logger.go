package logger

import (
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"time"
)

var ()

var (
	logFile    *os.File
	logger     *log.Logger
	logDir     string
	initOnce   sync.Once
	parentDir  string
)

func InitLogger(componentName string) {
	initOnce.Do(func() {
		// Get the absolute path to the 'Assignment' root directory dynamically.
		executablePath, err := os.Executable()
		if err != nil {
			log.Fatalf("Failed to get executable path: %v", err)
		}
		// Assuming 'Assignment' is the root directory
		parentDir = filepath.Dir(filepath.Dir(executablePath)) 

		// Create logs directory if it doesn't exist
		logsRoot := filepath.Join(parentDir, "logs")
		if err := os.MkdirAll(logsRoot, 0755); err != nil {
			log.Fatalf("Failed to create logs root directory: %v", err)
		}

		// Create a new log directory for the current run
		currentTime := time.Now().Format("02-01-2006_15:04") // DD-MM-YYYY_HH:MM
		logDir = filepath.Join(logsRoot, currentTime)
		if err := os.MkdirAll(logDir, 0755); err != nil {
			log.Fatalf("Failed to create runtime log directory: %v", err)
		}

		// Set up log file
		logFilePath := filepath.Join(logDir, fmt.Sprintf("%s.log", componentName))
		file, err := os.OpenFile(logFilePath, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
		if err != nil {
			log.Fatalf("Failed to open log file: %v", err)
		}
		logFile = file

		logger = log.New(logFile, "", 0) // No default flags, custom formatting below
	})
}

func CleanupLogDir() {
	if logFile != nil {
		logFile.Close()
	}

	if logDir != "" {
		// Only remove the directory if it's empty or contains only our log file
		_ = os.RemoveAll(logDir)
	}
}

func Logf(format string, v ...interface{}) {
	if logger == nil {
		log.Printf("Logger not initialized: "+format, v...)
		return
	}

	_, file, line, ok := runtime.Caller(1)
	if !ok {
		file = "???"
		line = 0
	}

	fileName := filepath.Base(file)
	
	message := fmt.Sprintf(format, v...)
	logEntry := fmt.Sprintf("%s[%d]: \"%s\"", fileName, line, strings.TrimSpace(message))

	logger.Println(logEntry)
}
