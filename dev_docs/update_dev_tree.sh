#!/bin/bash

# Define the output file and temporary file
output_file="dev_docs/dev_tree.json"
temp_new_file=$(mktemp)
temp_merged_file=$(mktemp)

# --- Cleanup function ---
cleanup() {
  rm -f "$temp_new_file" "$temp_merged_file"
}
trap cleanup EXIT # Ensure cleanup happens on script exit

# --- Check for jq ---
if ! command -v jq &> /dev/null; then
    echo "Error: 'jq' command not found. Please install jq to run this script." >&2
    exit 1
fi

# --- JSON Helper Functions (used by build_tree_json) ---

# Function to escape JSON strings (handles essential chars)
escape_json() {
    local input="$1"
    local escaped=${input//\\/\\\\} # Escape backslashes first
    escaped=${escaped//\"/\\\"} # Escape double quotes
    escaped=${escaped//$'\n'/\\n} # Replace newline
    escaped=${escaped//$'\r'/\\r} # Replace carriage return
    escaped=${escaped//$'\t'/\\t} # Replace tab
    printf "%s" "$escaped"
}

# Function to recursively build the JSON tree content (outputs formatted JSON objects/arrays)
# Arguments: $1 = directory to scan, $2 = current indentation string
build_tree_json() {
    local dir="$1"
    local indent="$2"
    local next_indent="${indent}  "
    local first_entry=true

    # --- Process files first ---
    local files_to_process=()
    while IFS= read -r -d $'\0' file; do
        files_to_process+=("$file")
    done < <(find "$dir" -maxdepth 1 -type f \( \
        -name '*.rs' -o \
        -name '*.toml' -o \
        -name '*.md' -o \
        -name '*.yaml' -o \
        -name '*.yml' -o \
        -name '*.sh' -o \
        -name '*.js' -o \
        -name '*.ts' -o \
        -name '.gitignore' -o \
        -name '.dockerignore' \
    \) -print0 | sort -z)

    for file in "${files_to_process[@]}"; do
        if [[ "$file" == *"target/"* ]] || [[ "$file" == *"$output_file"* ]] || [[ "$file" == *"$0"* ]] || (command -v git &> /dev/null && git check-ignore -q "$file"); then
            continue
        fi
        local filename=$(basename "$file")
        local filepath=${file#./}
        local escaped_filename=$(escape_json "$filename")
        local escaped_filepath=$(escape_json "$filepath")

        if [ "$first_entry" = false ]; then printf ",\n"; fi
        printf "%s{\n" "$indent"
        printf "%s\"name\": \"%s\",\n" "$next_indent" "$escaped_filename"
        printf "%s\"path\": \"%s\",\n" "$next_indent" "$escaped_filepath"
        printf "%s\"type\": \"file\",\n" "$next_indent"
        printf "%s\"summary\": \"\"\n" "$next_indent" # Default empty summary
        printf "%s}" "$indent"
        first_entry=false
    done

    # --- Process directories recursively ---
    local dirs_to_process=()
    while IFS= read -r -d $'\0' subdir; do
        dirs_to_process+=("$subdir")
    done < <(find "$dir" -maxdepth 1 -type d -print0 | sort -z)

    for subdir in "${dirs_to_process[@]}"; do
        local base_subdir=$(basename "$subdir")
        if [ "$subdir" == "." ] || [ "$subdir" == "$dir" ] || [[ "$base_subdir" == .* ]] || [[ "$subdir" == *"/target"* ]] || [[ "$subdir" == *"./dev_docs"* ]]; then
            continue
        fi
        if (command -v git &> /dev/null && git check-ignore -q "$subdir"); then
            continue
        fi
        local dirname=$base_subdir
        local subpath=${subdir#./}
        local escaped_dirname=$(escape_json "$dirname")
        local escaped_subpath=$(escape_json "$subpath")

        # Temporarily redirect stdout to capture children JSON
        local children_json=$(build_tree_json "$subdir" "$next_indent")

        if [ -n "$children_json" ]; then
            if [ "$first_entry" = false ]; then printf ",\n"; fi
            printf "%s{\n" "$indent"
            printf "%s\"name\": \"%s\",\n" "$next_indent" "$escaped_dirname"
            printf "%s\"path\": \"%s\",\n" "$next_indent" "$escaped_subpath"
            printf "%s\"type\": \"directory\",\n" "$next_indent"
            printf "%s\"children\": [\n" "$next_indent"
            printf "%s\n" "$children_json" # Output captured children
            printf "%s]\n" "$next_indent"
            printf "%s}" "$indent"
            first_entry=false
        fi
    done
}

# --- Generate New Tree (to temp file) ---
echo "Generating new file tree..."
{
  printf "{\n"
  printf "  \"name\": \".\",\n"
  printf "  \"path\": \".\",\n"
  printf "  \"type\": \"directory\",\n"
  printf "  \"children\": [\n"
  build_tree_json "." "    " # Initial indent for children array elements
  printf "\n  ]\n}\n"
} > "$temp_new_file"

echo "New tree generated in temporary file: $temp_new_file"

# --- Merge with Existing Data (if exists) ---
if [ -f "$output_file" ] && jq '.' "$output_file" > /dev/null 2>&1; then
  echo "Merging new tree with existing data from $output_file..."

  # Use the external jq script file
  jq --slurp -f dev_docs/merge_script.jq "$output_file" "$temp_new_file" > "$temp_merged_file"

  if [ $? -eq 0 ] && [ -s "$temp_merged_file" ]; then # Also check if merged file is not empty
    mv "$temp_merged_file" "$output_file"
    echo "Successfully merged and updated $output_file."
  else
    echo "Error: Failed to merge JSON files using jq or merge resulted in empty file. Original file $output_file remains unchanged." >&2
    # Optionally copy the temp files for debugging
    # cp "$temp_new_file" ./temp_new_file.json.err
    # cp "$output_file" ./output_file.json.err
    exit 1 # Exit if merge fails or is empty
  fi
else
  echo "Existing file $output_file not found or is not valid JSON. Overwriting with new tree."
  mv "$temp_new_file" "$output_file"
  # Attempt to format the new file just in case jq is available now
  jq '.' "$output_file" > "$temp_merged_file" && mv "$temp_merged_file" "$output_file" 2>/dev/null || true
  echo "Created new $output_file."
fi

exit 0 