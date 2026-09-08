/// A hand-written `.sublime-syntax` definition for KDL (https://kdl.dev), since
/// neither syntect's bundled syntaxes nor `two-face`'s extras ship one. Covers
/// both the v1 (bare `true`/`false`/`null`, `r"..."` raw strings) and v2
/// (`#true`/`#false`/`#null`, `#"..."#` raw strings) forms well enough for a
/// preview pane.
pub const KDL_SYNTAX_YAML: &str = r####"
name: KDL
scope: source.kdl
file_extensions:
  - kdl
contexts:
  main:
    - include: comments
    - match: '/-'
      scope: comment.slashdash.kdl
    - match: '\('
      scope: punctuation.section.type-annotation.begin.kdl
      push: type_annotation
    - include: strings
    - include: numbers
    - include: keywords
    - match: '[A-Za-z_][A-Za-z0-9_\-]*(?=\s*=)'
      scope: variable.parameter.property.kdl
    - match: '[A-Za-z_][A-Za-z0-9_\-]*'
      scope: entity.name.tag.kdl
    - match: '='
      scope: keyword.operator.assignment.kdl
    - match: '\{'
      scope: punctuation.section.block.begin.kdl
      push: main
    - match: '\}'
      scope: punctuation.section.block.end.kdl
      pop: true
    - match: ';'
      scope: punctuation.terminator.kdl

  comments:
    - match: '//.*$'
      scope: comment.line.double-slash.kdl
    - match: '/\*'
      scope: punctuation.definition.comment.begin.kdl
      push: block_comment

  block_comment:
    - match: '/\*'
      scope: punctuation.definition.comment.begin.kdl
      push: block_comment
    - match: '\*/'
      scope: punctuation.definition.comment.end.kdl
      pop: true
    - match: '[^*/]+|\*(?!/)|/(?!\*)'
      scope: comment.block.kdl

  type_annotation:
    - match: '\)'
      scope: punctuation.section.type-annotation.end.kdl
      pop: true
    - match: '[A-Za-z_][A-Za-z0-9_\-]*'
      scope: entity.name.type.kdl

  strings:
    - match: '"'
      scope: punctuation.definition.string.begin.kdl
      push: double_quoted_string
    - match: 'r(#*)"'
      scope: punctuation.definition.string.begin.kdl
      push: raw_string_v1
    - match: '(#+)"'
      scope: punctuation.definition.string.begin.kdl
      push: raw_string_v2

  double_quoted_string:
    - meta_scope: string.quoted.double.kdl
    - match: '\\u\{[0-9A-Fa-f]+\}|\\.'
      scope: constant.character.escape.kdl
    - match: '"'
      scope: punctuation.definition.string.end.kdl
      pop: true

  raw_string_v1:
    - meta_scope: string.quoted.other.raw.kdl
    - match: '"\1'
      scope: punctuation.definition.string.end.kdl
      pop: true

  raw_string_v2:
    - meta_scope: string.quoted.other.raw.kdl
    - match: '"\1'
      scope: punctuation.definition.string.end.kdl
      pop: true

  numbers:
    - match: '[-+]?0x[0-9A-Fa-f_]+'
      scope: constant.numeric.hex.kdl
    - match: '[-+]?0o[0-7_]+'
      scope: constant.numeric.octal.kdl
    - match: '[-+]?0b[01_]+'
      scope: constant.numeric.binary.kdl
    - match: '[-+]?[0-9][0-9_]*(\.[0-9][0-9_]*)?([eE][-+]?[0-9]+)?'
      scope: constant.numeric.kdl

  keywords:
    - match: '#?(true|false)\b'
      scope: constant.language.boolean.kdl
    - match: '#?null\b'
      scope: constant.language.null.kdl
    - match: '#(nan|-?inf)\b'
      scope: constant.language.kdl
"####;
