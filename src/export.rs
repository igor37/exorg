
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
    langs:          Vec<(String, String)>,
}

impl Exporter {
    pub fn from_file(filename: &String) -> Result<Self, ErrorKind> {
        let lines        = read_file(filename)?;
        let (src, langs) = Exporter::parse_src(&lines);
        Ok(Exporter {
            input_path:     filename.to_owned(),
            content_lines:  lines,
            src_blocks:     src,
            langs:          langs,
        })
    }

    pub fn export(&self, format: &String, block: &Option<String>) -> Result<(), ErrorKind> {
        let lower = format.to_lowercase();
        if lower.starts_with("pdf") {
            match lower.as_str() {
                "pdf"        => self.weave(false)?,
                "pdf-minted" => self.weave(true)?,
                _ => return Err(ErrorKind::InvalidOutputFormat),
            }
        } else {
            self.tangle(&lower, block)?;
        }
        Ok(())
    }

    fn parse_src(lines: &Vec<String>) -> (Vec<SrcBlock>, Vec<(String, String)>) {
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
                let mut parts: Vec<String> = line.split(" ")
                                            .map(|n| n.to_string())
                                            .collect();
                if parts.len() < 2 {
                    parts = vec![String::new(), "none".to_string()];
                }
                lang_name = Some(parts[1].clone());
                src = true;
            } else if line.starts_with("#+END_SRC") {
                src = false;
                src_blocks.push(SrcBlock {
                    name:  block_name.unwrap_or("none".to_string()),
                    lang:  lang_name.unwrap_or("none".to_string()).to_string(),
                    lines: block_lines.clone(),
                    dependencies: block_deps.clone(),
                });
                block_lines.clear();
                block_deps.clear();
                block_name = None;
                lang_name  = None;
            } else if line.starts_with("#+NAME:") {
                let mut trimmed = line.replace("#+NAME:", "");
                trimmed = trimmed.trim().to_string();
                block_name = Some(trimmed);
            } else if line.starts_with("#+DEPS:") {
                let mut trimmed = line.replace("#+DEPS:", "");
                trimmed = trimmed.trim().to_string();
                
                block_deps = trimmed.split(" ")
                    .filter(|n| n.len() > 0)
                    .map(|n| n.to_string())
                    .collect();
            } else if line.starts_with("#+SRC_LANG:") {
                let mut trimmed = line.replace("#+SRC_LANG:", "");
                trimmed = trimmed.trim().to_string();

                let lang: String = trimmed.split(" ")
                    .nth(0)
                    .unwrap_or("fail")
                    .to_string();
                let suffix: String = trimmed.split(" ")
                    .filter(|n| n.len() > 0)
                    .nth(1)
                    .unwrap_or("fail")
                    .to_string();
                langs.push( (lang, suffix) );
            } else if src {
                block_lines.push(line.to_owned());
            }
        }

        (src_blocks, langs)
    }

    /// PDF/LaTeX
    fn weave(&self, minted: bool) -> Result<(), ErrorKind> {
        let tex_cmd = "(org-latex-export-to-latex)";
        let pdf_cmd = "(org-latex-export-to-pdf)";
        let full_cmd = if minted {
            format!("(progn (setq org-confirm-babel-evaluate nil) {} (kill-emacs))", tex_cmd)
        } else {
            format!("(progn (setq org-confirm-babel-evaluate nil) {} (kill-emacs))", pdf_cmd)
        };

        let result = Command::new("emacs")
            .arg(&self.input_path)
            .arg("--batch")
            .arg("--eval")
            .arg(full_cmd)
            .output();

        // open .tex file and substitute verbatim blocks with minted src blocks,
        // then compile to pdf
        if minted {
            let tex_file_path = self.output_file_name(&"latex".to_string());
            let lines         = self.mint_tex( &read_file(&tex_file_path)? );
            write_file( &tex_file_path,
                        &lines )?;

            match Command::new("pdflatex")
                        .arg("-shell-escape")
                        .arg(&tex_file_path)
                        .output() {
                Err(_) => return Err(ErrorKind::PdfLatexCallFailed),
                Ok(_)  => {},
            }
        }
        // println!("{:?}", result);

        match result {
            Err(_) => Err(ErrorKind::EmacsCallFailed),
            Ok(_)  => Ok(()),
        }
    }

    /// Code extraction
    fn tangle(&self, target: &String, selected: &Option<String>) -> Result<(), ErrorKind> {
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
                let mut added = true;
                let mut relevant_blocks = vec![name.to_owned()];

                while added {
                    added = false;
                    for block in &self.src_blocks {
                        if relevant_blocks.contains(&block.name) {
                            for dep in &block.dependencies {
                                if !relevant_blocks.contains(&dep) {
                                    relevant_blocks.push(dep.to_string());
                                    added = true;
                                }
                            }
                        }
                    }
                }

                target_blocks = target_blocks.iter()
                                .filter(|b| relevant_blocks.contains(&b.name))
                                .map(|b| b.clone())
                                .collect();
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
            src_lines.pop();
        }

        let output_name = self.output_file_name(target);
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
            if line.contains("begin") && line.contains("{verbatim}") &&
                                lines[i+1] == self.src_blocks[src_idx].lines[0] {
                src_block = true;
                result.push(format!("\\begin{{minted}}{{{}}}",
                                    self.src_blocks[src_idx].lang));
            } else if line.contains("end") && line.contains("{verbatim}") &&
                                                                    src_block {
                src_block = false;                  
                src_idx += 1;
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
