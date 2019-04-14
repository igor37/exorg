# Exorg
Small CLI tool for exporting org-mode documents.

The main reason for its existence is that I like using Emacs not nearly as much as writing org-mode documents and wanted a simple 
command(and keybindings in Vim) for easily exporting the contents.

# Features
- LaTeX PDF export via Emacs or Pandoc
- Export of Python code within document into a Jupyter Notebook file
- Extraction of a selected(or all) source code blocks into source files(aka [tangling](https://en.wikipedia.org/wiki/Literate_programming#Workflow))

# Installation

With [Rust + Cargo installed](https://www.rust-lang.org/tools/install), clone the repository, then:
```
cd exorg
cargo install
```
Note that for PDF export Emacs or Pandoc need to be installed.

# Examples

The basic syntax is:
```
exorg <output format> <org file> [-b <block name>] [-o <output filename>]
```
(```-b``` is only relevant for code extraction, see below)

## PDF

For PDF export there are three options:
- **pdf**: Export to .tex and then .pdf via Emacs and pdflatex
- **pdf-minted**: Same as above but with syntax highlighting in source blocks. Requires the LaTeX package "minted"
- **pdf-pandoc**: Direct conversion to PDF via Pandoc. Behaviour not always consistent with Emacs

```
exorg pdf foo.org
exorg pdf-minted example.org
exorg pdf-pandoc bar.org -o baz.pdf
```

## Code Blocks

Provided source blocks in the document are correctly annotated, Exorg will be able to export all blocks into source files in the
appropriate format, or selected code blocks based on their name or programming language.

### Python

Tiny Python source block, annotated with the language "python" and the name "foo" in the standard org-mode syntax:
```
#+NAME: foo
#+BEGIN_SRC python
x = 37
print(x)
#+END_SRC
```

Now there are two ways to export this block - the first will export just this single one, the second one all Python blocks in the file.
In both cases, the result is a file called ```example.py```:
```
exorg python example.org -b foo
exorg python example.org
```

The ```.py``` file suffix is added automatically, the same works for a couple other popular languages. If not, the ```-n``` argument
can be used to specify the file name manually.

Exorg will autocomplete arguments for the ```-b``` flag if there is exactly one code block with a fitting name.

### Jupyter Notebook

This works very similar, just with a different format specifier:
```
exorg jupyter foo.org
# or ...
exorg jupyter foo.org -b block_name -o notebook.ipynb
```

### All code blocks

Exorg introduces an additional header argument for source blocks, ```#+FILE```(curently Emacs-compatible as it's ignored). It specifies a
path/filename to which the block should be exported. Multiple blocks with the same path end up in the same file. This makes it
possible to extract all source blocks simultaneously into new source files:
```
#+FILE: main.rs
#+BEGIN_SRC rust
fn main() {
    println!("Hello, World!");
}
#+END_SRC

#+FILE: foo.py
#+INCLUDE: src/code.py
```
Both code blocks(one of which is included from another file) will be written into two files with ```.``` as given format:
```
exorg . example.org
```

# License

Licensed under the MIT license.
