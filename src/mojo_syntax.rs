/// A hand-written `.sublime-syntax` definition for Mojo (https://www.modular.com/mojo),
/// since neither syntect's bundled syntaxes nor `two-face`'s extras ship one.
/// Mojo is a superset of Python, so this covers the shared Python surface
/// (comments, strings incl. f-strings/triple-quoted, numbers, decorators,
/// def/class) plus Mojo's own keywords (`fn`, `struct`, `trait`, `alias`,
/// `var`, `inout`, `owned`, `borrowed`, `raises`, ...) which is the part a
/// generic Python grammar would otherwise render as plain identifiers.
///
/// Simplification: `{ }` interpolation inside strings is highlighted for
/// every quoted string, not only ones prefixed `f`/`F`. A literal brace in a
/// plain string is rare enough in practice that this isn't worth doubling
/// every string context to track the prefix.
pub const MOJO_SYNTAX_YAML: &str = r####"
name: Mojo
scope: source.mojo
file_extensions:
  - mojo
  - "🔥"
contexts:
  main:
    - include: comments
    - match: '^\s*(@)([A-Za-z_][A-Za-z0-9_.]*)'
      captures:
        1: punctuation.definition.decorator.mojo
        2: entity.name.function.decorator.mojo
    - match: '\b(struct|trait|class)\s+([A-Za-z_][A-Za-z0-9_]*)'
      captures:
        1: keyword.declaration.mojo
        2: entity.name.type.mojo
    - match: '\b(fn|def)\s+([A-Za-z_][A-Za-z0-9_]*)'
      captures:
        1: keyword.declaration.mojo
        2: entity.name.function.mojo
    - match: '\b(fn|struct|trait|class|def)\b'
      scope: keyword.declaration.mojo
    - match: '\b(var|let|alias|inout|owned|borrowed|ref|raises|capturing|escaping)\b'
      scope: storage.modifier.mojo
    - match: '\b(if|elif|else|for|while|try|except|finally|with|as|import|from|return|yield|pass|break|continue|lambda|and|or|not|in|is|del|assert|raise|global|nonlocal|async|await|match|case)\b'
      scope: keyword.control.mojo
    - match: '\b(True|False|None)\b'
      scope: constant.language.mojo
    - match: '\b(self|Self|cls)\b'
      scope: variable.language.mojo
    - match: '->'
      scope: keyword.operator.arrow.mojo
    - match: '(:=|==|!=|<=|>=|<|>|\+|-|\*\*|\*|//|/|%|&|\||\^|~|<<|>>|=)'
      scope: keyword.operator.mojo
    - include: numbers
    - include: strings

  comments:
    - match: '#.*$'
      scope: comment.line.number-sign.mojo

  numbers:
    - match: '\b0[xX][0-9A-Fa-f_]+\b'
      scope: constant.numeric.hex.mojo
    - match: '\b0[oO][0-7_]+\b'
      scope: constant.numeric.octal.mojo
    - match: '\b0[bB][01_]+\b'
      scope: constant.numeric.binary.mojo
    - match: '\b[0-9][0-9_]*\.[0-9_]*([eE][+-]?[0-9]+)?[jJ]?\b'
      scope: constant.numeric.float.mojo
    - match: '\.[0-9][0-9_]*([eE][+-]?[0-9]+)?[jJ]?\b'
      scope: constant.numeric.float.mojo
    - match: '\b[0-9][0-9_]*[eE][+-]?[0-9]+[jJ]?\b'
      scope: constant.numeric.float.mojo
    - match: '\b[0-9][0-9_]*[jJ]?\b'
      scope: constant.numeric.mojo

  strings:
    - match: '([A-Za-z]{0,2})(""")'
      captures:
        1: storage.type.string.mojo
        2: punctuation.definition.string.begin.mojo
      push: triple_dq_string
    - match: "([A-Za-z]{0,2})(''')"
      captures:
        1: storage.type.string.mojo
        2: punctuation.definition.string.begin.mojo
      push: triple_sq_string
    - match: '([A-Za-z]{0,2})(")'
      captures:
        1: storage.type.string.mojo
        2: punctuation.definition.string.begin.mojo
      push: dq_string
    - match: "([A-Za-z]{0,2})(')"
      captures:
        1: storage.type.string.mojo
        2: punctuation.definition.string.begin.mojo
      push: sq_string

  triple_dq_string:
    - meta_scope: string.quoted.triple.double.mojo
    - match: '\\.'
      scope: constant.character.escape.mojo
    - match: '\{\{|\}\}'
      scope: constant.character.escape.mojo
    - match: '\{'
      scope: punctuation.section.interpolation.begin.mojo
      push: interpolation
    - match: '"""'
      scope: punctuation.definition.string.end.mojo
      pop: true

  triple_sq_string:
    - meta_scope: string.quoted.triple.single.mojo
    - match: '\\.'
      scope: constant.character.escape.mojo
    - match: '\{\{|\}\}'
      scope: constant.character.escape.mojo
    - match: '\{'
      scope: punctuation.section.interpolation.begin.mojo
      push: interpolation
    - match: "'''"
      scope: punctuation.definition.string.end.mojo
      pop: true

  dq_string:
    - meta_scope: string.quoted.double.mojo
    - match: '\\.'
      scope: constant.character.escape.mojo
    - match: '\{\{|\}\}'
      scope: constant.character.escape.mojo
    - match: '\{'
      scope: punctuation.section.interpolation.begin.mojo
      push: interpolation
    - match: '"'
      scope: punctuation.definition.string.end.mojo
      pop: true

  sq_string:
    - meta_scope: string.quoted.single.mojo
    - match: '\\.'
      scope: constant.character.escape.mojo
    - match: '\{\{|\}\}'
      scope: constant.character.escape.mojo
    - match: '\{'
      scope: punctuation.section.interpolation.begin.mojo
      push: interpolation
    - match: "'"
      scope: punctuation.definition.string.end.mojo
      pop: true

  interpolation:
    - match: '\}'
      scope: punctuation.section.interpolation.end.mojo
      pop: true
    - include: strings
    - include: numbers
    - match: '[A-Za-z_][A-Za-z0-9_]*'
      scope: variable.other.mojo
"####;
