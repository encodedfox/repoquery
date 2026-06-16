#!/usr/bin/env bash
#
# Generate diagrams from Mermaid source files
#
# This script converts .mmd files in docs/diagrams/ to PNG and SVG
# formats for use in documentation. Requires mermaid-cli (mmdc).
#
# Installation: npm install -g @mermaid-js/mermaid-cli
#
# Usage: ./scripts/generate-diagrams.sh

set -euo pipefail

DIAGRAM_DIR="docs/diagrams"
OUTPUT_DIR="generated-diagrams"
BACKUP_DIR="$OUTPUT_DIR/archive"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🎨 OmniDatum Diagram Generator${NC}"
echo "================================"

# Check for mermaid-cli
if ! command -v mmdc &> /dev/null; then
    echo -e "${RED}❌ mermaid-cli not found${NC}"
    echo ""
    echo "Install with:"
    echo "  npm install -g @mermaid-js/mermaid-cli"
    echo ""
    echo "Or with yarn:"
    echo "  yarn global add @mermaid-js/mermaid-cli"
    exit 1
fi

echo -e "${GREEN}✓${NC} mermaid-cli found: $(mmdc --version)"

# Create output directories
mkdir -p "$OUTPUT_DIR"
mkdir -p "$BACKUP_DIR"

# Backup existing diagrams
if ls "$OUTPUT_DIR"/*.png &> /dev/null; then
    echo ""
    echo -e "${YELLOW}📦 Backing up existing diagrams...${NC}"
    timestamp=$(date +%Y%m%d_%H%M%S)
    backup_target="$BACKUP_DIR/$timestamp"
    mkdir -p "$backup_target"
    cp "$OUTPUT_DIR"/*.png "$backup_target/" 2>/dev/null || true
    echo -e "${GREEN}✓${NC} Backed up to: $backup_target"
fi

# Count diagram files
diagram_count=$(find "$DIAGRAM_DIR" -name "*.mmd" | wc -l | tr -d ' ')
echo ""
echo "Found $diagram_count Mermaid diagram(s) to process"
echo ""

# Generate diagrams
success_count=0
error_count=0

for mmd_file in "$DIAGRAM_DIR"/*.mmd; do
    if [ -f "$mmd_file" ]; then
        filename=$(basename "$mmd_file" .mmd)
        
        echo -e "${BLUE}Processing${NC} $filename..."
        
        # Validate Mermaid syntax
        if mmdc -i "$mmd_file" -o /dev/null 2>&1 | grep -q "Success\|Generating single mermaid chart"; then
            echo -e "${GREEN}  ✓${NC} Syntax valid"
        else
            echo -e "${YELLOW}  ⚠${NC}  Warning: Possible syntax issues (continuing anyway)"
        fi
        
        # Generate PNG (for GitHub and README)
        if mmdc -i "$mmd_file" -o "$OUTPUT_DIR/$filename.png" \
                -w 1600 -H 1200 -b transparent 2>/dev/null; then
            echo -e "${GREEN}  ✓${NC} Generated PNG (1600x1200)"
        else
            echo -e "${RED}  ✗${NC} Failed to generate PNG"
            ((error_count++))
            continue
        fi
        
        # Generate SVG (for web and print)
        if mmdc -i "$mmd_file" -o "$OUTPUT_DIR/$filename.svg" \
                -b transparent 2>/dev/null; then
            echo -e "${GREEN}  ✓${NC} Generated SVG"
        else
            echo -e "${YELLOW}  ⚠${NC}  SVG generation failed (PNG still available)"
        fi
        
        ((success_count++))
        echo ""
    fi
done

# Summary
echo "================================"
if [ $success_count -eq $diagram_count ]; then
    echo -e "${GREEN}✨ Success!${NC} Generated $success_count diagram(s)"
elif [ $error_count -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Partial success:${NC} $success_count succeeded, $error_count failed"
else
    echo -e "${GREEN}✨ All diagrams generated successfully!${NC}"
fi

echo ""
echo "Output files:"
echo "  PNG: $OUTPUT_DIR/*.png"
echo "  SVG: $OUTPUT_DIR/*.svg"
echo ""
echo "Mermaid source:"
echo "  $DIAGRAM_DIR/*.mmd"
echo ""
echo -e "${BLUE}💡 Tip:${NC} Edit .mmd files and re-run this script to update diagrams"
echo -e "${BLUE}💡 Tip:${NC} View interactively at https://mermaid.live"