pub(crate) fn split_camel_case(name: &str) -> Vec<String> {
    let characters: Vec<char> = name.chars().filter(|character| *character != '_').collect();
    let mut words = Vec::new();
    let mut current = String::new();
    for (index, character) in characters.iter().enumerate() {
        let previous = index
            .checked_sub(1)
            .and_then(|before| characters.get(before));
        let next = characters.get(index + 1);
        let starts_word = previous.is_some_and(|previous| {
            (character.is_uppercase() && (previous.is_lowercase() || previous.is_ascii_digit()))
                || (character.is_uppercase()
                    && previous.is_uppercase()
                    && next.is_some_and(|next| next.is_lowercase()))
                || (character.is_ascii_digit() != previous.is_ascii_digit())
        });
        if starts_word && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        current.push(*character);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

pub(crate) fn word_forms(word: &str) -> Vec<String> {
    let lower = word.to_lowercase();
    let mut forms = vec![lower.clone(), format!("{lower}s"), format!("{lower}es")];
    if let Some(stem) = lower.strip_suffix("ies") {
        forms.push(format!("{stem}y"));
    }
    if let Some(stem) = lower.strip_suffix('y') {
        forms.push(format!("{stem}ies"));
    }
    if let Some(stem) = lower.strip_suffix("es") {
        forms.push(stem.to_owned());
    }
    if let Some(stem) = lower.strip_suffix('s') {
        forms.push(stem.to_owned());
    }
    forms
}
