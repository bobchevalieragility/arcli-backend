#!/bin/zsh

arc() {
  local response
  response=$(CLICOLOR_FORCE=1 command arc "$@")

  # Check for a special prefix to determine if the response is a command to execute
  if [[ "$response" == __EVAL__* ]]; then
    # Strip off the prefix and then execute the command
    eval "${response#__EVAL__}"
  else
    # Otherwise, just print the response
    echo "$response"
  fi
}
