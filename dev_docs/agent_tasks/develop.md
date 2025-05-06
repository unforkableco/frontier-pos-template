# Agent AI Task - Development request

When user asks for a modification of code, you must perform these tasks to create a development plan

1/ Look in dev_docs/dev_tree.json for pertinent information about the codebase and the files
2/ Create a new folder in dev_docs with a pertinent name to the task
3/ Create a development plan file in .md format where you list all the pertinent files to the task and broad steps to perform the task, also remind of the user prompt and your analysis of it in the document
4/ For every broad step, generate a detailed execution plan in the same .md file
5/ The last step should always be Result validation, it should include the creation of a script to validate the result (the script should not contain placeholders and must be executable without error directly) and the steps to perform by the user to validate manually.
6/ You must begin the development itself only after the user validated the development plan you provided and then follow the developpment plan carefully
7/ You can ask the user additional questions during the development process
8/ Once the user validates that the development is succesfully terminated, you must create another .md file next to the development plan that contains the summary of completed steps and the end result in terms of fuctionality added, removed or modified.

## Proper development techniques

1/ Try to make atomic changes to the code when possible, and rebuilding at every opportunity to make sure the change is correct. If the new changes aren't building, fix the issues one at a time and rebuild until done, then proceed with the rest of the development.
2/ When the codebase is using an external dependency, you can git clone the dependency in a temp folder in order to look for definitions and relevant code parts to help you understand. This must be done only after asking the user's permission

## Building instructions

The proper command to build an executable is "cargo build --release --features=testnet"

## Running tests instructions

The proper command to run the tests is "cargo test --features=testnet"

## Specific cargo check instructions

The proper command to run cargo check is "cargo check --features=testnet"

## Starting a node

The proper command to run a node in testnet mode is "./target/release/substrate --database auto --alice --dev

## Validation scripts

The proper way of writing a validation script is:

1/ The script must be in .sh format
2/ If the script requires a running node, it should run the node in background and terminate the node after the test
3/ The validation script should contain ample debug output
4/ The script must have the same name as the development task to perform