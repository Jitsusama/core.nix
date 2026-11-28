"""Read Rust source as code alone, for the rules that look for shapes in it.

A rule reading Rust by pattern takes a word in a string, or a bracket in a
comment, for code unless the two are told apart. This blanks out a file's
comments and fills its strings and characters, keeping each line where it
was, so a rule can match code without lexing it again.
"""

import re

# A raw string's opening, and a character literal, which a lifetime's `'a` isn't.
RAW_STRING = re.compile(r'b?r(#*)"')
CHARACTER = re.compile(r"'(\\.[^']*|[^\\'])'")


def rust_code(text: str) -> str:
    """Blank out a Rust file's comments, and fill its strings and characters, keeping lines.

    What's left is the code's words and brackets where they were, so a word,
    a brace or a comma in a string or a comment isn't taken for code, while
    a string still counts as something given.
    """
    out: list[str] = []
    index = 0
    while index < len(text):
        char = text[index]
        if text.startswith("//", index):
            end = text.find("\n", index)
            end = len(text) if end < 0 else end
            out.append(" " * (end - index))
            index = end
        elif text.startswith("/*", index):
            end = text.find("*/", index + 2)
            end = len(text) if end < 0 else end + 2
            out.append(re.sub(r"[^\n]", " ", text[index:end]))
            index = end
        elif raw := RAW_STRING.match(text, index):
            close = '"' + raw.group(1)
            end = text.find(close, index + len(raw.group(0)))
            end = len(text) if end < 0 else end + len(close)
            out.append(re.sub(r"[^\n]", "s", text[index:end]))
            index = end
        elif char == '"':
            end = index + 1
            while end < len(text) and text[end] != '"':
                end += 2 if text[end] == "\\" else 1
            out.append(re.sub(r"[^\n]", "s", text[index : end + 1]))
            index = end + 1
        elif literal := CHARACTER.match(text, index):
            out.append("c" * len(literal.group(0)))
            index += len(literal.group(0))
        else:
            out.append(char)
            index += 1
    return "".join(out)
