#!/bin/bash

set -e

OUTPUT_FILE="DOCUMENTATION.md"

# Clear the output file
> "$OUTPUT_FILE"

echo "Building documentation..."
echo "# Documentation" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Find all markdown files in the docs directory, sorted
# Sort by path to ensure consistent ordering
while IFS= read -r file; do
  if [ -f "$file" ]; then
    # Get relative path for the header
    relative_path="${file#docs/}"
    
    # Convert path to a nice header format
    # Replace slashes with " > " and remove .md extension
    header_title="${relative_path%.md}"
    header_title="${header_title//\//' > '}"
    
    echo "---" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    echo "## $header_title" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    
    # Add the file contents
    cat "$file" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
  fi
done < <(find docs -name "*.md" -type f | sort)

echo "Documentation built successfully!"
echo "Output: $OUTPUT_FILE"
