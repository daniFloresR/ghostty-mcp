#!/usr/bin/env python3
"""Tests for parse_docs.py."""
import unittest

import parse_docs


def parse(text):
    return parse_docs.parse_lines(text.splitlines(keepends=True))


class EnumDetection(unittest.TestCase):
    def test_bullet_list_of_backticked_values_is_enum(self):
        options = parse(
            "# The style of the cursor. Valid values are:\n"
            "#\n"
            "#   * `bar`\n"
            "#   * `block`\n"
            "#   * `underline`\n"
            "#   * `block_hollow`\n"
            "cursor-style = block\n"
        )
        self.assertEqual(options[0]["option_type"], "enum")
        self.assertEqual(
            options[0]["valid_values"], ["bar", "block", "underline", "block_hollow"]
        )

    def test_bullet_values_with_trailing_explanation_are_extracted(self):
        options = parse(
            "# Valid values:\n"
            "#\n"
            "#   * `true` - Enables the thing.\n"
            "#   * `false` - Disables the thing.\n"
            "#   * `always` - Always does the thing.\n"
            "thing-mode = \n"
        )
        self.assertEqual(options[0]["option_type"], "enum")
        self.assertEqual(options[0]["valid_values"], ["true", "false", "always"])

    def test_multiword_code_bullets_are_not_enum_values(self):
        options = parse(
            "# Examples:\n"
            "#\n"
            "#   * `font-family = Fira Code`\n"
            "#   * `font-family = Iosevka`\n"
            "some-option = \n"
        )
        self.assertEqual(options[0]["option_type"], "string")
        self.assertIsNone(options[0]["valid_values"])

    def test_boolean_default_wins_over_bullets(self):
        options = parse(
            "# Whether it blinks:\n"
            "#\n"
            "#   * `true`\n"
            "#   * `false`\n"
            "blinky = true\n"
        )
        self.assertEqual(options[0]["option_type"], "boolean")

    def test_single_bullet_is_not_enum(self):
        options = parse(
            "# See:\n"
            "#\n"
            "#   * `something`\n"
            "lonely = \n"
        )
        self.assertNotEqual(options[0]["option_type"], "enum")


class DescriptionInheritance(unittest.TestCase):
    GROUPED = (
        "# The font families to use. This long block documents the whole\n"
        "# font-family group of options.\n"
        "font-family = \n"
        "\n"
        "font-family-bold = \n"
        "font-family-italic = \n"
        "# The named font style.\n"
        "font-style = \n"
    )

    def test_first_option_gets_the_block(self):
        options = parse(self.GROUPED)
        self.assertIn("documents the whole", options[0]["description"])

    def test_undocumented_followers_inherit_group_description(self):
        options = parse(self.GROUPED)
        by_name = {o["name"]: o for o in options}
        self.assertIn("documents the whole", by_name["font-family-bold"]["description"])
        self.assertIn(
            "documents the whole", by_name["font-family-italic"]["description"]
        )

    def test_option_with_own_block_does_not_inherit(self):
        options = parse(self.GROUPED)
        by_name = {o["name"]: o for o in options}
        self.assertEqual(by_name["font-style"]["description"], "The named font style.")

    def test_no_empty_descriptions_in_groups(self):
        options = parse(self.GROUPED)
        for o in options:
            self.assertTrue(o["description"], f"{o['name']} has empty description")


class TypeInferencePrecedence(unittest.TestCase):
    def test_color_by_hex_description_keeps_special_values_not_enum(self):
        options = parse(
            "# The color of the cursor. Direct colors can be specified as either\n"
            "# hex (`#RRGGBB` or `RRGGBB`) or a named X11 color. Special values:\n"
            "#\n"
            "#   * `cell-foreground` - Match the cell foreground color.\n"
            "#   * `cell-background` - Match the cell background color.\n"
            "cursor-color = \n"
        )
        self.assertEqual(options[0]["option_type"], "color")
        self.assertEqual(
            options[0]["valid_values"], ["cell-foreground", "cell-background"]
        )

    def test_color_suffixed_name_without_hex_doc_is_strict_enum(self):
        options = parse(
            "# The color of the padding area of the window. Valid values are:\n"
            "#\n"
            "#   * `background` - The background color.\n"
            "#   * `extend` - Extend the background of the nearest cell.\n"
            "#   * `extend-always` - Always extend.\n"
            "window-padding-color = background\n"
        )
        self.assertEqual(options[0]["option_type"], "enum")
        self.assertEqual(
            options[0]["valid_values"], ["background", "extend", "extend-always"]
        )

    def test_comma_separated_flag_lists_are_not_strict_enums(self):
        options = parse(
            "# Flags to enable. The format of this is a list of flags to enable\n"
            "# separated by commas. If you prefix a flag with `no-` then it is\n"
            "# disabled.\n"
            "#\n"
            "#   * `hinting`\n"
            "#   * `force-autohint`\n"
            "#   * `monochrome`\n"
            "freetype-load-flags = hinting\n"
        )
        self.assertEqual(options[0]["option_type"], "string")
        self.assertEqual(
            options[0]["valid_values"], ["hinting", "force-autohint", "monochrome"]
        )

    def test_duration_options_do_not_become_enums_of_units(self):
        options = parse(
            "# How long the overlay is visible. The duration is specified as a\n"
            "# series of numbers followed by time units:\n"
            "#\n"
            "#   * `y` - years\n"
            "#   * `d` - days\n"
            "#   * `h` - hours\n"
            "resize-overlay-duration = 750ms\n"
        )
        self.assertEqual(options[0]["option_type"], "duration")
        self.assertIsNone(options[0]["valid_values"])

    def test_keybind_keeps_keybind_type_despite_bullets(self):
        options = parse(
            "# Bind a key. Special values:\n"
            "#\n"
            "#   * `ignore` - Do nothing.\n"
            "#   * `unbind` - Remove the binding.\n"
            "keybind = \n"
        )
        self.assertEqual(options[0]["option_type"], "keybind")

    def test_directory_options_with_special_values_stay_paths(self):
        options = parse(
            "# The directory to change to. Special values:\n"
            "#\n"
            "#   * `home` - The home directory.\n"
            "#   * `inherit` - The working directory of the launching process.\n"
            "working-directory = \n"
        )
        self.assertEqual(options[0]["option_type"], "path")
        self.assertEqual(options[0]["valid_values"], ["home", "inherit"])

    def test_bullet_with_multiple_backticked_values_extracts_all(self):
        options = parse(
            "# Whether to inject shell integration. Valid values:\n"
            "#\n"
            "#   * `none` - Do not do any automatic injection.\n"
            "#   * `detect` - Detect the shell based on the filename.\n"
            "#   * `bash`, `elvish`, `fish`, `zsh` - Use this specific shell.\n"
            "shell-integration = detect\n"
        )
        self.assertEqual(options[0]["option_type"], "enum")
        self.assertEqual(
            options[0]["valid_values"],
            ["none", "detect", "bash", "elvish", "fish", "zsh"],
        )

    def test_percentage_accepting_options_are_not_numbers(self):
        options = parse(
            "# The values can be integers (1, -1, etc.) or a percentage\n"
            "# (20%, -15%, etc.). In each case, the values represent the\n"
            "# amount to change the original value.\n"
            "adjust-cell-height = \n"
        )
        self.assertEqual(options[0]["option_type"], "string")

    def test_uppercase_and_assignment_bullets_are_not_enum_values(self):
        options = parse(
            "# The command to run. Looked up from `SHELL` or `passwd`:\n"
            "#\n"
            "#   * `SHELL`\n"
            "#   * `gtk-single-instance=false`\n"
            "#   * `passwd`\n"
            "command = \n"
        )
        self.assertNotEqual(options[0]["option_type"], "enum")


class OrphanBlocks(unittest.TestCase):
    def test_block_not_followed_by_option_is_discarded(self):
        # The docs sometimes print a comment block for an option whose key
        # line is omitted on the generating platform; the next option must
        # not absorb that orphan block.
        options = parse(
            "# The size of the thing. Long size documentation.\n"
            "# More size text.\n"
            "\n"
            "# The layer of the thing. Valid values are:\n"
            "#\n"
            "#   * `overlay`\n"
            "#   * `top`\n"
            "the-layer = top\n"
        )
        self.assertEqual(len(options), 1)
        self.assertNotIn("size", options[0]["description"])
        self.assertIn("layer", options[0]["description"])
        self.assertEqual(options[0]["valid_values"], ["overlay", "top"])


class StructureAndDeterminism(unittest.TestCase):
    SAMPLE = (
        "# A number option.\n"
        "number-opt = 42\n"
        "\n"
        "# A repeated option. This can be repeated.\n"
        "multi = a\n"
        "# A repeated option. This can be repeated.\n"
        "multi = b\n"
    )

    def test_duplicates_are_collapsed(self):
        options = parse(self.SAMPLE)
        names = [o["name"] for o in options]
        self.assertEqual(len(names), len(set(names)))

    def test_required_fields_present(self):
        options = parse(self.SAMPLE)
        required = {
            "name",
            "description",
            "default_value",
            "option_type",
            "valid_values",
            "category",
            "platform",
            "reloadable",
            "repeatable",
            "search_terms",
            "related_options",
        }
        for o in options:
            self.assertEqual(required - set(o.keys()), set())

    def test_output_is_deterministic(self):
        a = parse(self.SAMPLE)
        b = parse(self.SAMPLE)
        self.assertEqual(a, b)


if __name__ == "__main__":
    unittest.main()
