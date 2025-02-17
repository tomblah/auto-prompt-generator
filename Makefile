# Makefile

RELEASE_DIR := release
MONO_SCRIPT := $(RELEASE_DIR)/generate-prompt.sh

# List the scripts in the order they should appear.
# Order matters: helpers need to come before the main script.
SOURCES := \
    file-types.sh \
    get-git-root.sh \
    get-package-root.sh \
    filter-files-singular.sh \
    find-prompt-instruction.sh \
    extract-instruction-content.sh \
    assemble-prompt.sh \
    extract-types.sh \
    find-definition-files.sh \
    filter-files.sh \
    exclude-files.sh \
    diff-with-branch.sh \
    extract-enclosing-type.sh \
    find-referencing-files.sh \
    filter-substring-markers.sh \
    get-search-roots.sh \
    generate-prompt.sh

.PHONY: all clean release

all: release

release:
	@mkdir -p $(RELEASE_DIR)
	@echo "Creating monolithic script at $(MONO_SCRIPT)..."
	@# Start with a single shebang line.
	@echo "#!/bin/bash" > $(MONO_SCRIPT)
	@# Process each file in order.
	@for file in $(SOURCES); do \
	    echo "Processing $$file"; \
	    sed '/^#\!/d' $$file | sed '/source/ d' | \
	    sed '/if \[\[.*BASH_SOURCE\[0\].*\]\] *; *then/,/^[[:space:]]*fi[[:space:]]*$$/d' >> $(MONO_SCRIPT); \
	done
	@chmod +x $(MONO_SCRIPT)
	@echo "Monolithic script created successfully."

clean:
	@rm -rf $(RELEASE_DIR)
