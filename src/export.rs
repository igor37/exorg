
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
        if &lower == "pdf" {
            self.weave()?;
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
    fn weave(&self) -> Result<(), ErrorKind> {
        let lisp_cmd = "(progn (setq org-confirm-babel-evaluate nil) (org-latex-export-to-pdf) (kill-emacs))";
        let result = Command::new("emacs")
            .arg(&self.input_path)
            .arg("--batch")
            .arg("--eval")
            .arg(lisp_cmd)
            .output();

        // println!("{:?}", result);

        match result {
            Err(_)  => Err(ErrorKind::EmacsCallFailed),
            Ok(msg) => Ok(()),
        }
    }

    /// Code extraction
    fn tangle(&self, target: &String, selected: &Option<String>) -> Result<(), ErrorKind> {
        let mut src_lines = Vec::new();

        // collect relevant source code snippets
        let mut target_blocks: Vec<SrcBlock> = self.src_blocks.iter()
                                                .filter(|b| &b.lang == target)
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

        for block in target_blocks {
            src_lines.append(&mut block.lines.clone());
            src_lines.push(String::new());
        }
        src_lines.pop();

        let mut output_name = "output".to_string();
        for (lang, suffix) in &self.langs {
            if lang == target {
                output_name = format!("{}.{}", output_name, suffix);
            }
        }

        write_file(&output_name, &src_lines)?;
        Ok(())
    }
}
