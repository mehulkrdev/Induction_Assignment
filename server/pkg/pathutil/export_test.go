//go:build test

package pathutil

import "sync"

// ResetWorkspaceRootForTesting resets the workspace root cache for testing purposes.
// This function is only compiled into test binaries via the //go:build test tag.
func ResetWorkspaceRootForTesting() {
	_workspaceRootOnce = sync.Once{}
	_workspaceRoot = ""
	_workspaceRootErr = nil
}
