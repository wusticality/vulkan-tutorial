#!/bin/bash

# The shaders directory.
SHADER_DIR="../assets/shaders"

echo "Building shaders .."

# Compile all shader files.
find $SHADER_DIR -type f \( -name "*.frag" -o -name "*.vert" \) -print0 | \
    xargs -0 -I {} glslc {} -o {}.spv

echo "Done!"
