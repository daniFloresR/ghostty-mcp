#!/usr/bin/env python3
"""Parse Ghostty docs output into structured JSON."""
import sys
import json
import re

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

# Options known to be repeatable that the heuristic cannot detect from
# description text alone (they use different phrasing like "overwrite"
# or "map" instead of "repeat").
REPEATABLE_OVERRIDES = {'keybind', 'palette', 'custom-shader', 'custom-shader-animation'}

# Bullet list items of the form "* `value`" (after the leading "# " has been
# stripped from the comment lines), optionally followed by an explanation.
# A single bullet may list several values: "* `bash`, `fish`, `zsh` - ...".
# Values containing whitespace are code examples, not enum values.
ENUM_BULLET = re.compile(r'^\s*\*\s+`([^`\s]+)`')
BACKTICKED_VALUE = re.compile(r'`([^`\s]+)`')


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

    desc_lower = description.lower()

    # Bullet items are enum candidates, but only shaped like real config
    # values: uppercase tokens are env vars, and values containing '=' or
    # ':' are inline code examples or templates.
    candidates = []
    for line in description.split('\n'):
        if not ENUM_BULLET.match(line):
            continue
        # Only the head of the bullet (before the " - explanation") lists
        # values; the explanation may mention unrelated backticked words.
        head = line.split(' - ', 1)[0]
        for value in BACKTICKED_VALUE.findall(head):
            if re.fullmatch(r'[a-z0-9][a-z0-9_.-]*', value):
                candidates.append(value)
    if len(candidates) < 2:
        candidates = []
    info_values = candidates or None

    # Bullets only describe a closed value set when the option is not a
    # keybind/path/duration/color (there the bullets are special values or
    # units on top of an open domain) and not a comma-combinable flag list.
    is_keybind = name == 'keybind'
    is_path = name.endswith('-path') or name.endswith('-dir') or name.endswith('-directory')
    is_duration = 'duration' in desc_lower or ('millisecond' in desc_lower and 'value' in desc_lower)
    is_hex_color = 'hex' in desc_lower and ('rrggbb' in desc_lower or '#' in desc_lower)
    is_combinable = 'separated by comma' in desc_lower

    if candidates and not (is_keybind or is_path or is_duration or is_hex_color or is_combinable):
        return 'enum', candidates

    if is_hex_color:
        return 'color', info_values
    if name.endswith('-color') or name in ('foreground', 'background') or name.startswith('palette'):
        return 'color', info_values

    try:
        if default_value and default_value.replace('.', '', 1).replace('-', '', 1).isdigit():
            return 'number', None
    except Exception:
        pass
    if ('integer' in desc_lower or ('number' in desc_lower and 'between' in desc_lower)) \
            and 'percentage' not in desc_lower:
        return 'number', None

    if is_duration:
        return 'duration', None

    if is_keybind:
        return 'keybind', info_values

    if is_path:
        return 'path', info_values

    return 'string', info_values


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


def parse_lines(lines):
    """Parse raw `ghostty +show-config --default --docs` lines into options."""
    options = []
    current_comments = []
    # Consecutive undocumented options inherit the description of the group
    # they follow (the docs document e.g. font-family-bold/-italic with the
    # single block above font-family).
    last_description = ''

    for raw_line in lines:
        line = raw_line.rstrip('\n')

        if line.startswith('#'):
            current_comments.append(line)
            continue

        if line.strip() == '':
            # A blank line ends a comment block. The docs occasionally print
            # a block for an option whose key line is omitted on the
            # generating platform; without this reset the next option would
            # absorb that orphan block.
            current_comments = []
            continue

        # Option line: "name = value" or "name ="
        if '=' in line:
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

            if description:
                last_description = description
            else:
                description = last_description

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

    # Second pass: add related options
    all_names = [opt['name'] for opt in options]
    for opt in options:
        opt['related_options'] = find_related_options(opt['name'], all_names)

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

    return unique_options


def main():
    if len(sys.argv) < 3:
        print("Usage: parse_docs.py <raw_docs_path> <output_path>")
        sys.exit(1)

    raw_docs_path = sys.argv[1]
    output_path = sys.argv[2]

    with open(raw_docs_path, 'r') as f:
        lines = f.readlines()

    unique_options = parse_lines(lines)

    with open(output_path, 'w') as f:
        json.dump(unique_options, f, indent=2)

    print(f"Parsed {len(unique_options)} unique options")

    cats = {}
    for opt in unique_options:
        cats[opt['category']] = cats.get(opt['category'], 0) + 1
    print("\nCategories:")
    for cat, count in sorted(cats.items(), key=lambda x: (-x[1], x[0])):
        print(f"  {cat}: {count}")

    types = {}
    for opt in unique_options:
        types[opt['option_type']] = types.get(opt['option_type'], 0) + 1
    print("\nTypes:")
    for t, count in sorted(types.items(), key=lambda x: (-x[1], x[0])):
        print(f"  {t}: {count}")


if __name__ == '__main__':
    main()
