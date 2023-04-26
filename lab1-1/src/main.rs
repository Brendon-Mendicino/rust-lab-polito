use clap::Parser;

#[derive(Debug, Parser)]
struct Args  {
    #[arg()]
    input: String,
}


const SUBS_I : &str =
"àáâäæãåāăąçćčđďèéêëēėęěğǵḧîïíīįìıİłḿñńǹňôöòóœøōõőṕŕřßśšşșťțûüùúūǘůűųẃẍÿýžźż";
const SUBS_O: &str =
"aaaaaaaaaacccddeeeeeeeegghiiiiiiiilmnnnnoooooooooprrsssssttuuuuuuuuuwxyyzzz";


fn remove_diacritic(char: char) -> char {
    match SUBS_I.chars().position(|c| c == char) {
        Some(val) => SUBS_O.chars().take(val + 1).last().unwrap(),
        None => char,
    }
}

fn slugify(slug: String) -> String {
    let mut slugified = String::new();

    for mut char in slug.to_lowercase().chars() {
        
        char = remove_diacritic(char);

        let next_char = match char {
            'a' ..= 'z' => char,
            '0' ..=  '9' => char,
            _ => '-',
        };

        match slugified.chars().last() {
            Some(val) => if val == '-' && next_char == '-' {
                continue;
            },
            None => (),
        }

        slugified.push(next_char);
    }

    slugified
}


fn main() {
    let args = Args::parse();
    let mut s = SUBS_I.to_string();
    s.push_str("TEST123&&");
    s.push_str(&args.input);
    println!("{}", slugify(s));
}
