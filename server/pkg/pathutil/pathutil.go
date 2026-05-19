package pathutil

import (
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"
	"sync"

	"github.com/mehulkrdev/Assignment/server/pkg/logger"
)

var (
	_workspaceRoot     string
	_workspaceRootOnce sync.Once
	_workspaceRootErr  error
)

func FindWorkspaceRoot() (string, error) {
	_workspaceRootOnce.Do(func() {
		_workspaceRoot, _workspaceRootErr = findWorkspaceRootUncached()
	})
	return _workspaceRoot, _workspaceRootErr
}

func findWorkspaceRootUncached() (string, error) {
	initialCwd, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("failed to get current working directory: %w", err)
	}
	initialCwd = filepath.Clean(initialCwd)

	// 1. Check WORKSPACE_ROOT environment variable
	if envRoot := os.Getenv("WORKSPACE_ROOT"); envRoot != "" {
		logger.Logf("DEBUG: Attempting to resolve workspace root from WORKSPACE_ROOT environment variable: %s", envRoot)
		envRoot = filepath.Clean(envRoot)
		if isValidWorkspaceRoot(envRoot) {
			logger.Logf("Resolved workspace root via WORKSPACE_ROOT environment variable: %s", envRoot)
			return envRoot, nil
		}
		logger.Logf("WARNING: WORKSPACE_ROOT environment variable is set but invalid or does not point to a valid workspace root: %s", envRoot)
	}

	// 2. Check current working directory
	logger.Logf("DEBUG: Attempting to resolve workspace root from current working directory: %s", initialCwd)
	if isValidWorkspaceRoot(initialCwd) {
		logger.Logf("Resolved workspace root via current working directory: %s", initialCwd)
		return initialCwd, nil
	}

	// 3. Repository marker discovery by traversing up
	curr := initialCwd
	for {
		logger.Logf("DEBUG: Traversing up to find workspace root: %s", curr)
		if isValidWorkspaceRoot(curr) {
			logger.Logf("Resolved workspace root via repository marker discovery: %s", curr)
			return curr, nil
		}

		parent := filepath.Dir(curr)
		if parent == curr {
			break // Reached filesystem root
		}
		curr = parent
	}

	return "", fmt.Errorf("workspace root not found after all attempts (env, cwd, marker discovery)")
}

func isValidWorkspaceRoot(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		logger.Logf("DEBUG: Path does not exist or is inaccessible during workspace root validation: %s, error: %v", path, err)
		return false
	}
	if !info.IsDir() {
		logger.Logf("DEBUG: Path is not a directory during workspace root validation: %s", path)
		return false
	}

	// Check for strong repository markers (any-of semantics)
	markers := []string{".git", "go.mod", "Cargo.toml"}
	foundMarker := false
	for _, marker := range markers {
		markerPath := filepath.Join(path, marker)
		if s, err := os.Stat(markerPath); err == nil {
			// For .git, it can be a directory or a file (worktrees/submodules)
			if marker == ".git" && (s.IsDir() || s.Mode().IsRegular()) {
				logger.Logf("DEBUG: Found repository marker: %s at %s", marker, markerPath)
				foundMarker = true
				break
			} else if marker != ".git" && s.Mode().IsRegular() {
				logger.Logf("DEBUG: Found repository marker: %s at %s", marker, markerPath)
				foundMarker = true
				break
			}
		}
	}

	if !foundMarker {
		logger.Logf("DEBUG: No valid repository marker found in path: %s", path)
		return false
	}

	// Lightweight secondary validation: check for expected top-level directories (any-of semantics)
	// This is informative but non-blocking in production/container environments.
	expectedDirs := []string{"server", "third_party", "crates"}
	foundExpectedDir := false
	for _, dir := range expectedDirs {
		dirPath := filepath.Join(path, dir)
		if info, err := os.Stat(dirPath); err == nil && info.IsDir() {
			foundExpectedDir = true
			break
		}
	}

	if !foundExpectedDir {
		logger.Logf("DEBUG: Optional secondary validation did not find expected top-level directories in: %s. Proceeding with repository marker discovery.", path)
	}

	return true
}

// ResolveDataPath constructs and normalizes the absolute path to the data directory.
func ResolveDataPath(workspaceRoot string) (string, error) {
	if workspaceRoot == "" {
		return "", fmt.Errorf("workspaceRoot cannot be empty")
	}
	dataPath := filepath.Join(workspaceRoot, "data")
	dataPath = filepath.Clean(dataPath)
	dataPath, err := filepath.Abs(dataPath)
	if err != nil {
		return "", fmt.Errorf("failed to get absolute path for data directory: %w", err)
	}
	logger.Logf("Resolved data path: %s", dataPath)
	return dataPath, nil
}

// EnsureDataDir creates the data directory if it does not exist and verifies its state.
func EnsureDataDir(dataPath string) error {
	info, err := os.Stat(dataPath)
	if os.IsNotExist(err) {
		logger.Logf("Data directory does not exist, creating it: %s", dataPath)
		err = os.MkdirAll(dataPath, 0755)
		if err != nil {
			return fmt.Errorf("failed to create data directory %s: %w", dataPath, err)
		}
		logger.Logf("Data directory created successfully: %s", dataPath)
		info, err = os.Stat(dataPath) // Re-stat after creation
	}

	if err != nil {
		return fmt.Errorf("failed to stat data directory %s after creation attempt: %w", dataPath, err)
	}

	if !info.IsDir() {
		return fmt.Errorf("path %s exists but is not a directory", dataPath)
	}

	return nil
}

// GetThirdPartyPath returns the absolute path to the third_party directory.
func GetThirdPartyPath() (string, error) {
	root, err := FindWorkspaceRoot()
	if err != nil {
		return "", err
	}
	return filepath.Join(root, "third_party"), nil
}

// GetDataPath returns the absolute path to the data directory.
func GetDataPath() (string, error) {
	root, err := FindWorkspaceRoot()
	if err != nil {
		return "", err
	}
	dataPath, err := ResolveDataPath(root)
	if err != nil {
		return "", err
	}
	if err := EnsureDataDir(dataPath); err != nil {
		return "", err
	}
	return dataPath, nil
}

// AtomicWriteFile writes data to a file in an atomic fashion.
// It writes to a temporary file and then renames it to the final destination.
func AtomicWriteFile(filename string, data []byte, perm os.FileMode) error {
	dir := filepath.Dir(filename)
	tmpFile, err := ioutil.TempFile(dir, "*.tmp")
	if err != nil {
		return fmt.Errorf("failed to create temporary file: %w", err)
	}
	defer os.Remove(tmpFile.Name()) // Clean up temp file on exit

	if _, err := tmpFile.Write(data); err != nil {
		tmpFile.Close()
		return fmt.Errorf("failed to write to temporary file: %w", err)
	}

	if err := tmpFile.Sync(); err != nil { // Ensure data is flushed to disk
		tmpFile.Close()
		return fmt.Errorf("failed to sync temporary file: %w", err)
	}

	if err := tmpFile.Close(); err != nil {
		return fmt.Errorf("failed to close temporary file: %w", err)
	}

	if err := os.Chmod(tmpFile.Name(), perm); err != nil {
		return fmt.Errorf("failed to set permissions on temporary file: %w", err)
	}

	if err := os.Rename(tmpFile.Name(), filename); err != nil {
		return fmt.Errorf("failed to rename temporary file to final destination: %w", err)
	}

	return nil
}
