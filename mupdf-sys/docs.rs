use std::{cell::RefCell, collections::HashMap};

use bindgen::callbacks::{EnumVariantValue, ParseCallbacks};
use regex::{Regex, RegexBuilder};

#[derive(Debug)]
pub struct DocsCallbacks {
    types: Regex,
    full_names: RefCell<HashMap<String, String>>,
}

impl Default for DocsCallbacks {
    fn default() -> Self {
        Self {
            types: RegexBuilder::new("fz_[a-z_*]+")
                .case_insensitive(true)
                .build()
                .unwrap(),
            full_names: RefCell::default(),
        }
    }
}

impl ParseCallbacks for DocsCallbacks {
    fn item_name(&self, original_item_name: &str) -> Option<String> {
        self.full_names
            .borrow_mut()
            .insert(original_item_name.to_owned(), original_item_name.to_owned());
        None
    }

    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<String> {
        let enum_name = enum_name?;
        if enum_name.contains("unnamed at ") {
            return None;
        }

        let name = format!("{}_{}", enum_name, original_variant_name);
        self.full_names
            .borrow_mut()
            .insert(original_variant_name.to_owned(), name);
        None
    }

    fn process_comment(&self, comment: &str) -> Option<String> {
        let mut output = String::new();
        let mut newlines = 0;
        let mut arguments = false;

        for line in comment.split('\n') {
            let mut line = line.trim();
            if line.is_empty() {
                newlines += 1;
                continue;
            }

            let mut argument = false;
            if let Some(pline) = line.strip_prefix("@param") {
                line = pline;
                argument = true;
            }

            match newlines {
                _ if argument => output.push('\n'),
                0 => {}
                1 => output.push_str("<br>"),
                _ => output.push_str("\n\n"),
            };
            newlines = 0;

            if argument {
                if !arguments {
                    output.push_str("# Arguments\n");
                    arguments = true;
                }
                output.push_str("* ");
            }

            let line = line
                .replace('[', "\\[")
                .replace(']', "\\]")
                .replace('<', "\\<")
                .replace('>', "\\>")
                .replace("NULL", "`NULL`");
            let mut line = self.types.replace_all(&line, |c: &regex::Captures| {
                let name = &c[0];
                if name.contains('*') {
                    return format!("`{}`", name);
                }

                let full_names = self.full_names.borrow();
                if let Some(full_name) = full_names.get(name) {
                    return format!("[`{}`]({})", name, full_name);
                }

                if let Some(short_name) = name.strip_suffix("s") {
                    if let Some(full_name) = full_names.get(short_name) {
                        return format!("[`{}`]({})s", short_name, full_name);
                    }
                }

                format!("[`{}`]", name)
            });

            if let Some((first, rest)) = line.split_once(": ") {
                let mut new_line = String::new();

                for arg in first.split(", ") {
                    if arg.contains(|c: char| c.is_whitespace() || c == '`') {
                        new_line.clear();
                        break;
                    }

                    if !new_line.is_empty() {
                        new_line.push_str(", ");
                    }
                    new_line.push('`');
                    new_line.push_str(arg);
                    new_line.push('`');
                }

                if !new_line.is_empty() {
                    new_line.push_str(": ");
                    new_line.push_str(rest);
                    line = new_line.into();
                }
            }

            output.push_str(&line);

            newlines += 1;
        }
        Some(output)
    }
}
