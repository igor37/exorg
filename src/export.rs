
use std::process::Command;

use error::ErrorKind;
use file::{read_file, write_file};

#[derive(Clone, Debug)]
struct SrcBlock {
    pub name:  String,
    pub lang:  String,
    pub lines: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Exporter {
    input_path:     String,
    content_lines:  Vec<String>,
    src_blocks:     Vec<SrcBlock>,
    // langs: (<language name>, <file prefix>)
    langs:          Vec<(String, String)>,
}

impl Exporter {
    pub fn from_file(filename: &String) -> Result<Self, ErrorKind> {
        let lines        = read_file(filename)?;
        let (src, langs) = Exporter::extract_src(&lines)?;
        Ok(Exporter {
            input_path:     filename.to_owned(),
            content_lines:  lines,
            src_blocks:     src,
            langs:          langs,
        })
    }

    fn src_blocks(&self) -> &Vec<SrcBlock> {
        &self.src_blocks
    }

    fn langs(&self) -> &Vec<(String, String)> {
        &self.langs
    }

    pub fn export(&self, format: &String, block: &Option<String>,
                                out: &Option<String>) -> Result<(), ErrorKind> {
        let lower = format.to_lowercase();
        if lower == "pdf" || lower == "pdf-minted" {
            match lower.as_str() {
                "pdf"        => self.weave(false)?,
                "pdf-minted" => self.weave(true)?,
                _ => unreachable!(),
            }
        } else {
            self.tangle(&lower, block, out)?;
        }
        Ok(())
    }

    fn extract_src(lines: &Vec<String>) -> Result<(Vec<SrcBlock>, Vec<(String, String)>), ErrorKind> {
        let mut lang_name   = None;
        let mut block_name  = None;
        let mut block_lines = Vec::new();
        let mut block_deps  = Vec::new();
        let mut langs       = Vec::new();
        let mut src_blocks  = Vec::new();

        let mut src = false;
        for full_line in lines {
            let line = full_line.replace("\n", "");

            if line.starts_with("#+BEGIN_SRC") {
                lang_name = Exporter::parse_begin_src(&line);
                src = true;
            } else if line.starts_with("#+END_SRC") {
                src_blocks.push(SrcBlock {
                    name:  block_name.unwrap_or("".to_string()),
                    lang:  lang_name.unwrap_or("".to_string()).to_string(),
                    lines: block_lines.clone(),
                    dependencies: block_deps.clone(),
                });
                block_lines.clear();
                block_deps.clear();
                block_name = None;
                lang_name  = None;
                src = false;
            } else if line.starts_with("#+NAME:") {
                block_name = Some(Exporter::parse_name(&line));
            } else if line.starts_with("#+DEPS:") {
                block_deps = Exporter::parse_deps(&line);
            } else if line.starts_with("#+SRC_LANG:") {
                langs.push(Exporter::parse_src_lang(&line));
            } else if line.starts_with("#+INCLUDE:") {
                Exporter::parse_include(&line, &mut src_blocks,
                                        &mut langs, block_name, block_deps)?;
                block_name = None;
                block_deps = Vec::new();
            } else if src {
                block_lines.push(line.to_owned());
            }
        }
        Ok((src_blocks, langs))
    }

    fn parse_begin_src(line: &String) -> Option<String> {
        let lang_str: Option<String> = line.split(" ")
                                    // skip the "#+BEGIN_SRC" phrase
                                    .skip(1)
                                    // discard empty strings which occur if
                                    // multiple spaces are inbetween args
                                    .filter(|n| n.len() > 0)
                                    // flags like -i and -n not relevant here
                                    .filter(|n| !(n.starts_with("-") &&
                                                  n.len() == 2))
                                    .nth(0)
                                    .map(|s| s.to_string());
        lang_str
    }

    fn parse_name(line: &String) -> String {
        let trimmed = line.replace("#+NAME:", "");
        trimmed.trim().to_string()
    }

    fn parse_deps(line: &String) -> Vec<String> {
        let mut trimmed = line.replace("#+DEPS:", "");
        trimmed = trimmed.trim().to_string();
                
        trimmed.split(" ")
                .filter(|n| n.len() > 0)
                .map(|n| n.to_string())
                .collect()
    }

    fn parse_src_lang(line: &String) -> (String, String) {
        let mut trimmed = line.replace("#+SRC_LANG:", "");
        trimmed = trimmed.trim().to_string();

        let mut args = trimmed.split(" ")
                        .filter(|n| n.len() > 0);
        let lang:   String = args.nth(0).unwrap_or("fail").to_string();
        let suffix: String = args.nth(1).unwrap_or("fail").to_string();
        (lang, suffix)
    }

    fn parse_include(line: &String, src_blocks: &mut Vec<SrcBlock>,
                     langs: &mut Vec<(String, String)>,
                     block_name: Option<String>, block_deps: Vec<String>) 
                                                    -> Result<(), ErrorKind> {
        let args = line.split(" ")
                    .filter(|n| n.len() > 0)
                    .map(|n| n.to_string())
                    .collect::<Vec<String>>();
        let len = args.len();
        
        if len == 2 { // no optional arguments => .org file
            let included_filename = &args[1];
            let exporter = Exporter::from_file(included_filename)?;
            let mut new_src_blocks = exporter.src_blocks().clone();
            let mut new_langs      = exporter.langs().clone();
            src_blocks.append(&mut new_src_blocks);
            langs.append(&mut new_langs);
        } else if len == 4 &&
                  &args[2] == "src" { // src import
            let included_filename = &args[1];
            let lang  = args[3].clone();
            let lines = read_file(included_filename)?;
            let name  = match block_name {
                Some(n) => n,
                None    => String::new(),
            };

            src_blocks.push(SrcBlock {
                name:  name,
                lang:  lang,
                lines: lines,
                dependencies: block_deps,
            });
        }
        // other variants if includes are assumed to contain no src code
        Ok(())
    }

    /// PDF/LaTeX
    fn weave(&self, minted: bool) -> Result<(), ErrorKind> {
        let full_cmd = "(progn (setq org-confirm-babel-evaluate nil) (org-latex-export-to-latex) (kill-emacs))";

        match Command::new("emacs")
                    .arg(&self.input_path)
                    .arg("--batch")
                    .arg("--eval")
                    .arg(full_cmd)
                    .output() {
            Err(_) => return Err(ErrorKind::EmacsCallFailed),
            Ok(_)  => {},
        }

        let tex_file_path = self.output_file_name(&"latex".to_string());
        // open .tex file and substitute verbatim blocks with minted src blocks,
        // then compile to pdf
        if minted {
            let lines = self.mint_tex( &read_file(&tex_file_path)? );
            write_file( &tex_file_path,
                        &lines )?;
        }

        match Command::new("pdflatex")
                    .arg("-shell-escape")
                    // .arg(&out_dir_arg)
                    .arg(&tex_file_path)
                    .output() {
            Err(_) => return Err(ErrorKind::PdfLatexCallFailed),
            Ok(m)  => {
                // if no PDF was produced due to a fatal error, print the
                // error message
                let out = format!("{}", m.stdout.iter()
                                                    .map(|n| *n as char)
                                                    .collect::<String>());
                if out.contains("no output PDF file produced") {
                    println!("ERROR occurred. Log:\n{}", out);
                }
            },
        }
        Ok(())
    }

    /// Code extraction
    fn tangle(&self, target: &String, selected: &Option<String>,
                                out: &Option<String>) -> Result<(), ErrorKind> {
        let mut src_lines = Vec::new();

        // collect relevant source code snippets
        let mut target_blocks: Vec<SrcBlock> = self.src_blocks.iter()
                                                .filter(|b| &b.lang == target ||
                                                        (&b.lang == "python" &&
                                                         target  == "jupyter"))
                                                .map(|b| b.clone())
                                                .collect();
        
        match selected {
            Some(name) => {
                let mut selected_name = name.to_string();
                // look for the selected block and figure out if the user
                // provided the incomplete name of the selected block, expecting
                // autocompletion.
                //
                // remember all blocks which have the given name as prefix
                let mut prefixes = Vec::new();
                let mut matches  = Vec::new();
                for bi in 0..self.src_blocks.len() {
                    if self.src_blocks[bi].name.starts_with(selected_name.as_str()) {
                        prefixes.push(bi);
                        if &self.src_blocks[bi].name == &selected_name {
                            matches.push(bi);
                        }
                    }
                }
                // if the number of exact matches exceeds 1 we don't know which
                // block to select
                if matches.len() > 1 {
                    return Err(ErrorKind::AmbiguousCodeBlockName);
                } else if matches.len() == 0 { // no exact match -> autocomplete
                    if prefixes.len() > 1 {
                        return Err(ErrorKind::AmbiguousCodeBlockName);
                    } else if prefixes.len() == 0 {
                        return Err(ErrorKind::CodeBlockNotFound);
                    } else {
                        selected_name = self.src_blocks[prefixes[0]].name.to_owned();
                    }
                } // else name already is the correct, full name

                let mut added = true;
                let mut relevant_block_names = vec![selected_name.to_owned()];

                while added {
                    added = false;
                    for block in &self.src_blocks {
                        if relevant_block_names.contains(&block.name) {
                            for dep in &block.dependencies {
                                if !relevant_block_names.contains(&dep) {
                                    relevant_block_names.push(dep.to_string());
                                    added = true;
                                }
                            }
                        }
                    }
                }

                // collect all src blocks, ordering them with respect to their
                // dependencies.
                let mut new_insertion;
                let mut blocks = target_blocks.clone();
                target_blocks.clear();
                let mut inserted_block_names = Vec::new();
                loop {
                    new_insertion = false;
                    for block in &blocks {
                        if inserted_block_names.contains(&block.name) ||
                          !relevant_block_names.contains(&block.name) {
                            continue;
                        }

                        let mut dependencies_met = true;
                        for dependency in &block.dependencies {
                            if !inserted_block_names.contains(&dependency) {
                                dependencies_met = false;
                                break;
                            }
                        }

                        if dependencies_met {
                            inserted_block_names.push(block.name.clone());
                            target_blocks.push(block.clone());
                            new_insertion = true;
                        }
                    }

                    if inserted_block_names.len() >= relevant_block_names.len() {
                        break;
                    }
                    if !new_insertion {
                        return Err(ErrorKind::UnsatisfiableDependencies);
                    }
                }
            },
            None => {},
        }

        if target == "jupyter" {
            src_lines = Exporter::build_jupyter_notebook(&target_blocks);
        } else {
            for block in target_blocks {
                src_lines.append(&mut block.lines.clone());
                src_lines.push(String::new());
            }
            src_lines.pop(); // last line is always empty
        }

        let output_name = match out {
            Some(s) => s.to_string(),
            None    => self.output_file_name(target),
        };
        write_file(&output_name, &src_lines)?;
        Ok(())
    }

    fn mint_tex(&self, lines: &Vec<String>) -> Vec<String> {
        let mut result = Vec::new();
        let mut src_idx = 0;
        let mut src_block = false;
        let mut pkg = false;

        for i in 0..lines.len() {
            let line = lines[i].clone();
            // add import for minted package in header area
            if !pkg && line.starts_with("\\usepackage") {
                result.push("\\usepackage{minted}".to_string());
                pkg = true;
            }
            // replace all verbatim src blocks
            if src_idx < self.src_blocks.len() &&
                    line.contains("begin") && line.contains("{verbatim}") &&
                    lines[i+1].trim().contains(self.src_blocks[src_idx].lines[0].trim()) {

                result.push(format!("\\begin{{minted}}{{{}}}",
                                    self.src_blocks[src_idx].lang));
                src_block = true;
                src_idx += 1;
            } else if line.contains("end") && line.contains("{verbatim}") &&
                                                                    src_block {
                src_block = false;                  
                result.push("\\end{minted}".to_string());
            } else {    // keep the rest of the code identical
                result.push(line);
            }
        }

        result
    }

    fn output_file_name(&self, target: &String) -> String {
        let input_file = self.input_path.split('/').last().unwrap();
        let prefix     = input_file.split('.').nth(0).unwrap();

        match target.as_str() {
            ""                        => format!("{}", prefix),
            "awk"                     => format!("{}.awk", prefix),
            "bash" | "sh" | "shell"   => format!("{}.sh", prefix),
            "c"                       => format!("{}.c", prefix),
            "cpp" | "c++"             => format!("{}.cpp", prefix),
            "csharp" | "c#" | "cs"    => format!("{}.cs", prefix),
            "css"                     => format!("{}.css", prefix),
            "d"                       => format!("{}.d", prefix),
            "emacs-lisp"              => format!("{}.el", prefix),
            "go"                      => format!("{}.go", prefix),
            "html"                    => format!("{}.html", prefix),
            "java"                    => format!("{}.java", prefix),
            "js"                      => format!("{}.js", prefix),
            "json"                    => format!("{}.json", prefix),
            "julia"                   => format!("{}.jl", prefix),
            "jupyter"                 => format!("{}.ipynb", prefix),
            "latex"                   => format!("{}.tex", prefix),
            "lua"                     => format!("{}.lua", prefix),
            "markdown"                => format!("{}.md", prefix),
            "ocaml"                   => format!("{}.ml", prefix),
            "perl"                    => format!("{}.pl", prefix),
            "php"                     => format!("{}.php", prefix),
            "prolog"                  => format!("{}.pl", prefix),
            "python"                  => format!("{}.py", prefix),
            "r"                       => format!("{}.r", prefix),
            "ruby"                    => format!("{}.rb", prefix),
            "rust"                    => format!("{}.rs", prefix),
            "sql"                     => format!("{}.sql", prefix),
            "toml"                    => format!("{}.toml", prefix),
            "yaml"                    => format!("{}.yml", prefix),
            // if unknown, check if the suffix was defined in the input file
            _ => {
                for (lang, suffix) in &self.langs {
                    if lang == target {
                        return format!("{}.{}", prefix, suffix);
                    }
                }
                prefix.to_string()
            },
        }
    }

    /// Generate syntax for a jupyter notebook(aka json) file.
    /// Only exports Python code, no Markdown blocks.
    fn build_jupyter_notebook(blocks: &Vec<SrcBlock>) -> Vec<String> {
        let mut clines = Vec::new();
        clines.push("{".to_string());
        // write cells
        clines.push(" \"cells\": [".to_string());

        for block in blocks {
            clines.push("  {".to_string());
            clines.push("   \"cell_type\": \"code\",".to_string());
            clines.push("   \"execution_count\": null,".to_string());
            clines.push("   \"metadata\": {},".to_string());
            clines.push("   \"outputs\": [],".to_string());
            clines.push("   \"source\": [".to_string());

            let len = block.lines.len();
            for k in 0..len {
                let escaped = block.lines[k].replace("\\", "\\\\")
                                            .replace("\"", "\\\"")
                                            .replace("\t", "    ");
                let line = if k < len-1 {
                    format!("    \"{}\\n\",", escaped)
                } else {
                    format!("    \"{}\\n\"", escaped)
                };
                clines.push(line);
            }

            let clen = clines.len();
            clines[clen-1] = clines[clen-1].replace("\\n", "");

            clines.push("   ]".to_string());
            clines.push("  },".to_string());
        }

        // the } of the last cell shouldn't be followed by a comma
        let clen = clines.len();
        clines[clen-1] = "  }".to_string();

        // write metadata
        clines.push(" ],".to_string());
        clines.push(" \"metadata\": {".to_string());
        clines.push("  \"kernelspec\": {".to_string());
        clines.push("   \"display_name\": \"Python 3\",".to_string());
        clines.push("   \"language\": \"python\",".to_string());
        clines.push("   \"name\": \"python3\"".to_string());
        clines.push("  },".to_string());
        clines.push("  \"language_info\": {".to_string());
        clines.push("   \"codemirror_mode\": {".to_string());
        clines.push("    \"name\": \"ipython\",".to_string());
        clines.push("    \"version\": 3".to_string());
        clines.push("   },".to_string());
        clines.push("   \"file_extension\": \".py\",".to_string());
        clines.push("   \"mimetype\": \"text/x-python\",".to_string());
        clines.push("   \"name\": \"python\",".to_string());
        clines.push("   \"nbconvert_exporter\": \"python\",".to_string());
        clines.push("   \"pygments_lexer\": \"ipython3\",".to_string());
        clines.push("   \"version\": \"3.6.4\"".to_string());
        clines.push("  }".to_string());
        clines.push(" },".to_string());
        clines.push(" \"nbformat\": 4,".to_string());
        clines.push(" \"nbformat_minor\": 2".to_string());
        clines.push("}".to_string());

        clines
    }
        
}
