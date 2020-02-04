
use std::process::Command;

use error::ErrorKind;
use file::{read_file, write_file};

#[derive(Copy, Clone, Debug)]
enum PdfOpt {
    Emacs,
    EmacsMinted,
    Pandoc,
}

#[derive(Clone, Debug)]
struct SrcBlock {
    pub name:  String,
    pub lang:  String,
    pub lines: Vec<String>,
    pub dependencies: Vec<String>,
    pub filename: Option<String>,
}

#[derive(Clone)]
struct FileContent {
    pub name:  String,
    pub lines: Vec<String>,
}

impl FileContent {
    fn new(name: &String) -> Self {
        FileContent {
            name: name.clone(),
            lines: Vec::new()
        }
    }

    /// Writes its lines into the file at the path stored in 'name' if 'lines'
    /// is not empty
    fn write_content(&self) -> Result<bool, ErrorKind> {
        if self.lines.len() > 0 {
            write_file(&self.name, &self.lines)?;
            return Ok(true);
        }
        Ok(false)
    }
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
                        out_filename: &Option<String>) -> Result<(), ErrorKind> {

        let lower_format = format.to_lowercase();
        if lower_format == "pdf" || lower_format.starts_with("pdf-") {
            match lower_format.as_str() {
                "pdf"        => self.weave(PdfOpt::Emacs)?,
                "pdf-minted" => self.weave(PdfOpt::EmacsMinted)?,
                "pdf-pandoc" => self.weave(PdfOpt::Pandoc)?,
                _ => unreachable!(),
            }
        } else {
            self.tangle(&lower_format, block, out_filename)?;
        }
        Ok(())
    }

    fn extract_src(lines: &Vec<String>) -> Result<(Vec<SrcBlock>, Vec<(String, String)>), ErrorKind> {
        let mut lang_name   = None;
        let mut block_name  = None;
        let mut block_file  = None;
        let mut block_lines = Vec::new();
        let mut block_deps  = Vec::new();
        let mut langs       = Vec::new();
        let mut src_blocks  = Vec::new();

        let mut src = false;
        for full_line in lines {
            let line = full_line.replace("\n", "");

            if line.starts_with("#+BEGIN_SRC") {
                let tup = Exporter::parse_begin_src(&line);
                lang_name  = tup.0;
                block_file = tup.1;
                src = true;
            } else if line.starts_with("#+END_SRC") {
                src_blocks.push(SrcBlock {
                    name:  block_name.unwrap_or("".to_string()),
                    lang:  lang_name.unwrap_or("".to_string()).to_string(),
                    lines: block_lines.clone(),
                    dependencies: block_deps.clone(),
                    filename: block_file.clone(),
                });
                block_lines.clear();
                block_deps.clear();
                block_name = None;
                block_file = None;
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
                block_file = None;
                block_deps = Vec::new();
            } else if src {
                block_lines.push(line.to_owned());
            }
        }
        Ok((src_blocks, langs))
    }

    fn parse_begin_src(line: &String) -> (Option<String>, Option<String>) {
        let metadata = line.split(" ")
                            // skip the "#+BEGIN_SRC" phrase
                            .skip(1)
                            // discard empty strings which occur if
                            // multiple spaces are inbetween args
                            .filter(|n| n.len() > 0)
                            // flags like -i and -n not relevant here
                            .filter(|n| !(n.starts_with("-") &&
                                          n.len() == 2));

        let remaining: Vec<String> = metadata.map(|s| s.to_string()).collect();
        let lang_str: Option<String> = if remaining.len() > 0 {
            Some(remaining[0].clone())
        } else { None };
        let mut filename: Option<String> = None;

        for i in 0..remaining.len() {
            if remaining[i] == ":tangle" && remaining.len() > i+1 {
                match remaining[i+1].as_str() {
                    "no"  => {},
                    "yes" => {},
                    name  => filename = Some(name.to_string()),
                }
                break;
            }
        }

        (lang_str, filename)
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
                     block_name: Option<String>, block_deps: Vec<String>) -> Result<(), ErrorKind> {
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
        } else if len >= 4 &&
                  &args[2] == "src" { // src import
            let included_filename = &args[1];
            let lang  = args[3].clone();
            let lines = read_file(included_filename)?;

            let block_file = if args.len() >= 6 && args[4] == ":tangle" {
                Some(args[5].clone())
            } else { None };


            let name  = match block_name {
                Some(n) => n,
                None    => String::new(),
            };

            src_blocks.push(SrcBlock {
                name:  name,
                lang:  lang,
                lines: lines,
                dependencies: block_deps,
                filename: block_file,
            });
        }
        // other variants of includes are assumed to contain no src code
        Ok(())
    }

    /// PDF/LaTeX
    fn weave(&self, pdf_opt: PdfOpt) -> Result<(), ErrorKind> {
        let tex_file_path = self.output_file_name(&"latex".to_string());

        match pdf_opt {
            PdfOpt::Emacs => {
                self.call_emacs()?;
                self.call_latex(&tex_file_path)?;
            },
            PdfOpt::EmacsMinted => {
                self.call_emacs()?;
                // open .tex file and substitute verbatim blocks with minted src blocks,
                // then compile to pdf
                let lines = self.mint_tex( &read_file(&tex_file_path)? );
                write_file( &tex_file_path, &lines )?;
                self.call_latex(&tex_file_path)?;
            },
            PdfOpt::Pandoc => self.call_pandoc()?,
        }

        Ok(())
    }

    fn call_emacs(&self) -> Result<(), ErrorKind> {
        let full_cmd = "(progn (setq org-confirm-babel-evaluate nil) (org-latex-export-to-latex) (kill-emacs))";

        match Command::new("emacs")
                    .arg(&self.input_path)
                    .arg("--batch")
                    .arg("--eval")
                    .arg(full_cmd)
                    .output() {
            Err(_) => return Err(ErrorKind::EmacsCallFailed),
            Ok(_)  => return Ok(()),
        }
    }

    fn call_pandoc(&self) -> Result<(), ErrorKind> {
        let path = if self.input_path.ends_with(".org") {
            let mut p = self.input_path.clone();
            let len = p.chars().count();
            p.truncate(len - 4);
            p
        } else {
            self.input_path.clone()
        };
        match Command::new("pandoc")
                    .arg("-f")
                    .arg("org")
                    .arg("-t")
                    .arg("latex")
                    .arg(&self.input_path)
                    .arg("-o")
                    .arg(format!("{}.pdf", &path))
                    .arg("--pdf-engine-opt=-shell-escape")
                    .arg("--toc")
                    .output() {
            Err(_) => return Err(ErrorKind::PandocCallFailed),
            Ok(_)  => return Ok(()),
        }
    }

    fn call_latex(&self, path: &String) -> Result<(), ErrorKind> {
        match Command::new("pdflatex")
                    .arg("-shell-escape")
                    .arg(path)
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
                return Ok(());
            },
        }
    }

    /// Code extraction
    fn tangle(&self, target: &String, selected: &Option<String>,
                        out_filename: &Option<String>) -> Result<(), ErrorKind> {
        let generic_out_name = match out_filename {
            Some(s) => s.to_string(),
            None    => self.output_file_name(target),
        };

        let mut files = Vec::new();
        let fallback_file = FileContent::new(&generic_out_name);
        files.push(fallback_file);

        // collect relevant source code snippets
        //
        // check for "." export mode as this means skipping language comparisons
        let mut target_blocks: Vec<SrcBlock> = if target == "." {
            self.src_blocks.clone()
        } else {
            self.src_blocks.iter()
                .filter(|b| &b.lang == target ||
                        (&b.lang == "python" &&
                         target  == "jupyter"))
                .map(|b| b.clone())
                .collect()
        };
        
        match selected {
            Some(name) => {
                self.select_blocks(name, &mut target_blocks)?;
            },
            None => {},
        }

        if target == "jupyter" {
            files.push(FileContent {
                name:  generic_out_name,
                lines: Exporter::build_jupyter_notebook(&target_blocks)
            });
        } else {
            if target == "." {
                Exporter::cp_src_to_files(&mut target_blocks, &mut files);
            } else { // just export into a single file
                for block in target_blocks {
                    if block.lines.len() > 0 {
                        files[0].lines.append(&mut block.lines.clone());
                        files[0].lines.push(String::new());
                    }
                }
            }
        }

        for file in files {
            file.write_content()?;
        }
        Ok(())
    }

    fn cp_src_to_files(target_blocks: &mut Vec<SrcBlock>, files: &mut Vec<FileContent>) {
        // copy lines of each src block into corresponding FileContent
        // instances, creating them on the go if necessary
        for block in target_blocks {
            let mut opt = None;
            // look if there's already a FileContent instance for this path
            match &block.filename {
                Some(f) => {
                    for fi in 0..files.len() {
                        if &files[fi].name == f {
                            opt = Some(fi);
                            break;
                        }
                    }
                },
                None    => {
                    opt = Some(0);
                }
            }
            // get the index of the FileContent instance, one way or another
            let idx = match opt {
                None => {
                    // unwrap() should be perfectly fine here as the
                    // "None" case is handled right above
                    files.push(FileContent::new(&block.clone().filename.unwrap()));
                    files.len()-1
                },
                Some(i) => i,
            };

            files[idx].lines.append(&mut block.lines.clone());
            files[idx].lines.push(String::new());
        }
    }

    fn select_blocks(&self, name: &String,
                    target_blocks: &mut Vec<SrcBlock>) -> Result<(), ErrorKind>{

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
        let blocks = target_blocks.clone();
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
            // replace all verbatim src blocks with a specified language
            if src_idx < self.src_blocks.len() &&
                line.contains("begin") && line.contains("{verbatim}") &&
                    self.src_blocks[src_idx].lines.len() > 0 &&
                    lines[i+1].trim().contains(self.src_blocks[src_idx].lines[0].trim()) &&
                    self.src_blocks[src_idx].lang != "" {

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
            "emacs-lisp" | "elisp"    => format!("{}.el", prefix),
            "go"                      => format!("{}.go", prefix),
            "html"                    => format!("{}.html", prefix),
            "java"                    => format!("{}.java", prefix),
            "js"                      => format!("{}.js", prefix),
            "json"                    => format!("{}.json", prefix),
            "julia"                   => format!("{}.jl", prefix),
            "jupyter"                 => format!("{}.ipynb", prefix),
            "latex"                   => format!("{}.tex", prefix),
            "lisp"                    => format!("{}.lisp", prefix),
            "lua"                     => format!("{}.lua", prefix),
            "markdown" | "md"         => format!("{}.md", prefix),
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
