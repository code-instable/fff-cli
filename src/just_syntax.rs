/// A hand-written `.sublime-syntax` definition for justfiles (https://just.systems),
/// since neither syntect's bundled syntaxes nor `two-face`'s extras ship one.
/// Covers comments, `:=` assignments, `set`/`alias`/`import`/`mod` directives,
/// `[attribute]` lines, recipe headers (name, parameters, dependencies) and
/// `{{ interpolation }}` inside recipe bodies. Recipe bodies are otherwise left
/// as plain shell text rather than guessing at shell syntax line by line, since
/// a wrong guess (e.g. a colon inside an echoed string) would misparse the rest
/// of the file.
pub const JUST_SYNTAX_YAML: &str = r####"
name: Just
scope: source.just
file_extensions:
  - just
  - justfile
  - Justfile
  - .justfile
contexts:
  main:
    - include: comments
    - match: '^\['
      scope: punctuation.section.attribute.begin.just
      push: attribute
    - match: '^\s*\b(export|set|alias|import\??|mod)\b'
      scope: keyword.other.just
    - match: '^([A-Za-z_][A-Za-z0-9_-]*)(?=\s*:=)'
      scope: variable.other.just
    - match: ':='
      scope: keyword.operator.assignment.just
    - match: '^(@?[A-Za-z_][A-Za-z0-9_-]*)(?=[^:\n]*:(?!=))'
      scope: entity.name.function.just
      push: recipe_header
    - include: strings

  comments:
    - match: '#.*$'
      scope: comment.line.number-sign.just

  attribute:
    - match: '\]'
      scope: punctuation.section.attribute.end.just
      pop: true
    - match: '[A-Za-z_][A-Za-z0-9_-]*'
      scope: support.function.attribute.just
    - include: strings

  recipe_header:
    - match: '\$?[A-Za-z_][A-Za-z0-9_-]*(?=\s*[:=+])'
      scope: variable.parameter.just
    - match: '\*'
      scope: keyword.operator.just
    - match: '='
      scope: keyword.operator.just
    - include: strings
    - match: ':'
      scope: punctuation.separator.just
      set: recipe_deps
    - match: '$'
      set: recipe_body

  recipe_deps:
    - match: '[A-Za-z_][A-Za-z0-9_-]*'
      scope: support.function.just
    - match: '\('
      scope: punctuation.section.group.begin.just
      push: recipe_dep_args
    - match: '$'
      set: recipe_body

  recipe_dep_args:
    - match: '\)'
      scope: punctuation.section.group.end.just
      pop: true
    - match: '='
      scope: keyword.operator.just
    - include: strings
    - match: '[A-Za-z_][A-Za-z0-9_-]*'
      scope: variable.parameter.just

  recipe_body:
    - match: '^(?=\S)'
      pop: true
    - match: '#.*$'
      scope: comment.line.number-sign.just
    - match: '\{\{'
      scope: punctuation.section.interpolation.begin.just
      push: interpolation
    - include: body_strings

  interpolation:
    - match: '\}\}'
      scope: punctuation.section.interpolation.end.just
      pop: true
    - include: strings
    - match: '[A-Za-z_][A-Za-z0-9_-]*'
      scope: variable.other.just

  strings:
    - match: '"'
      scope: punctuation.definition.string.begin.just
      push: dq_string
    - match: "'"
      scope: punctuation.definition.string.begin.just
      push: sq_string
    - match: '`'
      scope: punctuation.definition.string.begin.just
      push: backtick_string

  dq_string:
    - meta_scope: string.quoted.double.just
    - match: '\\.'
      scope: constant.character.escape.just
    - match: '"'
      scope: punctuation.definition.string.end.just
      pop: true

  sq_string:
    - meta_scope: string.quoted.single.just
    - match: "'"
      scope: punctuation.definition.string.end.just
      pop: true

  backtick_string:
    - meta_scope: string.interpolated.backtick.just
    - match: '`'
      scope: punctuation.definition.string.end.just
      pop: true

  # Same three string forms as above, but recognizing `{{ interpolation }}`
  # inside the string. Used only within recipe bodies, where interpolation
  # is meaningful (e.g. `echo "hello {{name}}"`) - a plain `strings` include
  # elsewhere would treat `{{`/`}}` in e.g. a variable's default value as
  # literal text, which is correct there since interpolation isn't valid
  # outside a recipe body.
  body_strings:
    - match: '"'
      scope: punctuation.definition.string.begin.just
      push: dq_body_string
    - match: "'"
      scope: punctuation.definition.string.begin.just
      push: sq_body_string
    - match: '`'
      scope: punctuation.definition.string.begin.just
      push: backtick_body_string

  dq_body_string:
    - meta_scope: string.quoted.double.just
    - match: '\\.'
      scope: constant.character.escape.just
    - match: '\{\{'
      scope: punctuation.section.interpolation.begin.just
      push: interpolation
    - match: '"'
      scope: punctuation.definition.string.end.just
      pop: true

  sq_body_string:
    - meta_scope: string.quoted.single.just
    - match: '\{\{'
      scope: punctuation.section.interpolation.begin.just
      push: interpolation
    - match: "'"
      scope: punctuation.definition.string.end.just
      pop: true

  backtick_body_string:
    - meta_scope: string.interpolated.backtick.just
    - match: '\{\{'
      scope: punctuation.section.interpolation.begin.just
      push: interpolation
    - match: '`'
      scope: punctuation.definition.string.end.just
      pop: true
"####;
