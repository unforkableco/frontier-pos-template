# Agent AI Task - Development request

When user asks for a modification of code, you must perform these tasks to create a development plan

1/ Look in dev_docs/dev_tree.json for pertinent information about the codebase and the files
2/ Create a new folder in dev_docs with a pertinent name to the task
3/ Create a development plan file in .md format where you list all the pertinent files to the task and broad steps to perform the task
4/ For every broad step, generate a detailed execution plan in the same .md file
5/ The last step should always be Result validation, it should include if possible the creation of a script to validate the result, if not, the steps to perform by the user to validate the task. If a script can be written, you must write the code
6/ You must begin the development itself only after the user validated the development plan you provided and then follow the developpment plan carefully

## Building instructions

The proper command to build an executable is "cargo build --release --features=testnet"

## Starting a node

The proper command to run a node in testnet mode is "./target/release/substrate --database auto --alice --dev

## Validation scripts

The proper way of writing a validation script is:

1/ The script must be in .sh format
2/ If the script requires a running node, it should run the node in background and terminate the node after the test
3/ The validation script should contain ample debug output
4/ The script must have the same name as the development task to perform