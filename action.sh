#!/bin/bash

COMMIT_MSG="${1:-chore: fix action}"
BRANCH="${2:-actions}"

mkdir -p .logs

echo "Message: \"$COMMIT_MSG\""

# Commit and push
git commit -m "$COMMIT_MSG"
git push

echo "Waiting for the new workflow run to start..."

# Simple polling mechanism to wait for the new workflow run
MAX_ATTEMPTS=10
ATTEMPT=0
WORKFLOW_RUN_ID=""
while [ -z "$WORKFLOW_RUN_ID" ] && [ $ATTEMPT -lt $MAX_ATTEMPTS ]; do
  sleep 10 # wait for 10 seconds before checking
  WORKFLOW_RUN_ID=$(gh run list --branch="$BRANCH" --limit=1 --json databaseId | jq -r '.[0].databaseId')
  ((ATTEMPT=ATTEMPT+1))
done

if [ -z "$WORKFLOW_RUN_ID" ]; then
  echo "Failed to find a new workflow run after $MAX_ATTEMPTS attempts"
  exit 1
fi

echo "New workflow run ID: $WORKFLOW_RUN_ID"

# Wait for the workflow to complete (optional, you can also stream directly if you know it's started)
echo "Waiting for workflow to complete..."
gh run watch "$WORKFLOW_RUN_ID"

echo "Workflow completed!"
gh run view "$WORKFLOW_RUN_ID" --web &
# Fetch and save the logs for the workflow run ID to a file
echo "Saving logs for workflow run ID: $WORKFLOW_RUN_ID to workflow_logs.txt"
gh run view "$WORKFLOW_RUN_ID" --log > .logs/workflow_logs.txt
