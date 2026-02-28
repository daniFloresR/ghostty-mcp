#!/usr/bin/env python3
"""Parse Ghostty docs output into structured JSON."""
import sys
import json
import re

def main():
    if len(sys.argv) < 3:
        print("Usage: parse_docs.py <raw_docs_path> <output_path>")
        sys.exit(1)

    raw_docs_path = sys.argv[1]
    output_path = sys.argv[2]

    with open(raw_docs_path, 'r') as f:
        lines = f.readlines()

    options = []
    current_comments = []

    # Category inference based on option name prefixes
    CATEGORY_MAP = {
        'font-': 'font',
        'cursor-': 'cursor',
        'mouse-': 'mouse',
        'background': 'appearance',
        'foreground': 'appearance',
        'selection-': 'selection',
        'palette': 'color',
        'bold-color': 'color',
        'faint-opacity': 'color',
        'window-': 'window',
        'gtk-': 'gtk',
        'clipboard-': 'clipboard',
        'scrollback-': 'scrollback',
        'link-': 'link',
        'keybind': 'keybind',
        'shell-integration': 'shell-integration',
        'command': 'shell',
        'working-directory': 'shell',
        'config-file': 'general',
        'confirm-close-surface': 'general',
        'quit-after-last-window-closed': 'general',
        'initial-command': 'shell',
        'wait-after-command': 'shell',
        'abnormal-command-exit-runtime': 'shell',
        'title': 'window',
        'class': 'window',
        'x11-': 'x11',
        'osc-color-report-format': 'compatibility',
        'image-storage-limit': 'image',
        'copy-on-select': 'clipboard',
        'click-repeat-interval': 'mouse',
        'desktop-notifications': 'general',
        'minimum-contrast': 'appearance',
        'adjust-': 'font',
        'alpha-blending': 'appearance',
        'grapheme-': 'font',
        'freetype-': 'font',
        'theme': 'appearance',
        'auto-update': 'general',
        'term': 'compatibility',
        'enquiry-response': 'compatibility',
        'macos-': 'macos',
        'linux-': 'linux',
        'resize-': 'window',
        'focus-follows-mouse': 'mouse',
        'quick-terminal-': 'quick-terminal',
        'swift-': 'compatibility',
        'app-id': 'window',
        'custom-shader': 'shader',
        'async-backend': 'general',
        'adw-toolbar-style': 'gtk',
    }

    def infer_category(name, description):
        if name in CATEGORY_MAP:
            return CATEGORY_MAP[name]
        for prefix, cat in CATEGORY_MAP.items():
            if name.startswith(prefix):
                return cat
        desc_lower = description.lower()
        if any(w in desc_lower for w in ['color', 'colour', 'palette']):
            return 'color'
        if any(w in desc_lower for w in ['font', 'glyph', 'typeface']):
            return 'font'
        if any(w in desc_lower for w in ['window', 'split', 'tab']):
            return 'window'
        if any(w in desc_lower for w in ['keybind', 'shortcut', 'key ']):
            return 'keybind'
        if any(w in desc_lower for w in ['mouse', 'click', 'scroll']):
            return 'mouse'
        if any(w in desc_lower for w in ['clipboard', 'paste', 'copy']):
            return 'clipboard'
        return 'general'

    def infer_type(name, default_value, description):
        if default_value in ('true', 'false'):
            return 'boolean', None

        valid_values = []
        enum_pattern = re.compile(r'#\s+\*\s+`([^`]+)`')
        for line in description.split('\n'):
            m = enum_pattern.search(line)
            if m:
                valid_values.append(m.group(1))

        if valid_values and len(valid_values) >= 2:
            return 'enum', valid_values

        desc_lower = description.lower()
        if 'hex' in desc_lower and ('rrggbb' in desc_lower or '#' in desc_lower):
            return 'color', None
        if name.endswith('-color') or name in ('foreground', 'background') or name.startswith('palette'):
            return 'color', None

        try:
            if default_value and default_value.replace('.', '', 1).replace('-', '', 1).isdigit():
                return 'number', None
        except:
            pass
        if 'integer' in desc_lower or ('number' in desc_lower and 'between' in desc_lower):
            return 'number', None

        if 'duration' in desc_lower or ('millisecond' in desc_lower and 'value' in desc_lower):
            return 'duration', None

        if name == 'keybind':
            return 'keybind', None

        if name.endswith('-path') or name.endswith('-dir') or name.endswith('-directory'):
            return 'path', None

        return 'string', None

    def infer_platform(name, description):
        platforms = []
        desc_lower = description.lower()
        if 'only supported on macos' in desc_lower or 'only on macos' in desc_lower or 'macos only' in desc_lower:
            platforms.append('macos')
        elif 'only supported on linux' in desc_lower or 'only on linux' in desc_lower or 'linux only' in desc_lower:
            platforms.append('linux')
        elif name.startswith('macos-'):
            platforms.append('macos')
        elif name.startswith('linux-') or name.startswith('gtk-') or name.startswith('adw-') or name.startswith('x11-'):
            platforms.append('linux')
        return platforms if platforms else None

    # Options known to be repeatable that the heuristic cannot detect from
    # description text alone (they use different phrasing like "overwrite"
    # or "map" instead of "repeat").
    REPEATABLE_OVERRIDES = {'keybind', 'palette', 'custom-shader', 'custom-shader-animation'}

    def is_repeatable(name, description):
        if name in REPEATABLE_OVERRIDES:
            return True
        desc_lower = description.lower()
        return 'repeated' in desc_lower or 'repeatable' in desc_lower or 'can be repeated' in desc_lower

    def is_reloadable(description):
        desc_lower = description.lower()
        if 'requires a full' in desc_lower and 'restart' in desc_lower:
            return False
        if 'changed at runtime' in desc_lower or 'changing this value at runtime' in desc_lower:
            return True
        if 'reloading' in desc_lower:
            return True
        return True

    def generate_search_terms(name, description):
        terms = set()
        for part in name.split('-'):
            if len(part) > 2:
                terms.add(part)

        synonym_map = {
            'background-opacity': ['transparent', 'transparency', 'alpha', 'see-through', 'opacity'],
            'font-family': ['typeface', 'font-name', 'text-font'],
            'font-size': ['text-size', 'font-scale', 'character-size'],
            'cursor-style': ['caret', 'cursor-shape', 'cursor-type'],
            'cursor-color': ['caret-color'],
            'window-padding-x': ['margin', 'border', 'padding-horizontal'],
            'window-padding-y': ['margin', 'border', 'padding-vertical'],
            'window-decoration': ['titlebar', 'chrome', 'window-frame'],
            'scrollback-limit': ['history', 'buffer-size', 'scroll-history'],
            'clipboard-read': ['paste-access'],
            'clipboard-write': ['copy-access'],
            'theme': ['color-scheme', 'colorscheme', 'dark-mode', 'light-mode'],
            'keybind': ['keyboard-shortcut', 'hotkey', 'key-mapping', 'binding'],
            'shell-integration': ['shell-setup', 'prompt'],
            'background': ['bg-color'],
            'foreground': ['fg-color', 'text-color'],
            'bold-color': ['bold-text-color'],
            'selection-foreground': ['selection-text-color', 'highlight-text'],
            'selection-background': ['selection-color', 'highlight-color'],
            'minimum-contrast': ['contrast-ratio', 'readability'],
            'mouse-hide-while-typing': ['auto-hide-mouse', 'cursor-auto-hide'],
            'confirm-close-surface': ['confirm-close', 'close-confirmation'],
            'copy-on-select': ['auto-copy', 'selection-copy'],
            'window-title-font-family': ['titlebar-font'],
            'quit-after-last-window-closed': ['auto-quit', 'close-app'],
            'macos-titlebar-style': ['titlebar-appearance', 'native-titlebar'],
            'link-url': ['clickable-links', 'hyperlinks'],
            'custom-shader': ['glsl', 'shader-effect', 'visual-effect'],
            'background-image': ['wallpaper', 'bg-image'],
        }

        if name in synonym_map:
            terms.update(synonym_map[name])

        return sorted(list(terms))

    def find_related_options(name, all_names):
        prefix = name.rsplit('-', 1)[0] if '-' in name else name
        related = []
        for other_name in all_names:
            if other_name != name and other_name.startswith(prefix + '-'):
                related.append(other_name)
        # Limit to avoid huge lists
        return related[:10] if related else None

    i = 0
    while i < len(lines):
        line = lines[i].rstrip('\n')

        if line.startswith('#'):
            current_comments.append(line)
            i += 1
            continue

        if line.strip() == '':
            i += 1
            continue

        # Option line: "name = value" or "name ="
        if '=' in line and not line.startswith('#'):
            parts = line.split('=', 1)
            name = parts[0].strip()
            default_value = parts[1].strip()

            desc_lines = []
            for cl in current_comments:
                if cl == '#':
                    desc_lines.append('')
                elif cl.startswith('# '):
                    desc_lines.append(cl[2:])
                elif cl.startswith('#'):
                    desc_lines.append(cl[1:])
            description = '\n'.join(desc_lines).strip()

            option_type, valid_values = infer_type(name, default_value, description)
            platform = infer_platform(name, description)

            option = {
                'name': name,
                'description': description,
                'default_value': default_value,
                'option_type': option_type,
                'valid_values': valid_values,
                'category': infer_category(name, description),
                'platform': platform,
                'reloadable': is_reloadable(description),
                'repeatable': is_repeatable(name, description),
                'search_terms': generate_search_terms(name, description),
            }

            options.append(option)
            current_comments = []

        i += 1

    # Second pass: add related options
    all_names = [opt['name'] for opt in options]
    for opt in options:
        related = find_related_options(opt['name'], all_names)
        opt['related_options'] = related

    # Deduplicate: some options appear multiple times (e.g. font-family variants)
    # Keep the first occurrence with the longest description
    seen = {}
    unique_options = []
    for opt in options:
        if opt['name'] in seen:
            # Keep the one with longer description
            existing_idx = seen[opt['name']]
            if len(opt['description']) > len(unique_options[existing_idx]['description']):
                unique_options[existing_idx] = opt
        else:
            seen[opt['name']] = len(unique_options)
            unique_options.append(opt)

    with open(output_path, 'w') as f:
        json.dump(unique_options, f, indent=2)

    print(f"Parsed {len(unique_options)} unique options (from {len(options)} total)")

    cats = {}
    for opt in unique_options:
        cats[opt['category']] = cats.get(opt['category'], 0) + 1
    print("\nCategories:")
    for cat, count in sorted(cats.items(), key=lambda x: -x[1]):
        print(f"  {cat}: {count}")

    types = {}
    for opt in unique_options:
        types[opt['option_type']] = types.get(opt['option_type'], 0) + 1
    print("\nTypes:")
    for t, count in sorted(types.items(), key=lambda x: -x[1]):
        print(f"  {t}: {count}")

if __name__ == '__main__':
    main()
