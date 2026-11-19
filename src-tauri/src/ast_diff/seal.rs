use super::model::ParsedFile;
use super::tokens::structural_hash;

struct Hashes {
    full: u64,
    signature: u64,
    body: u64,
    shallow: u64,
}

/// Computes the structural fingerprints of every declaration once all tokens are known.
pub fn seal(file: &mut ParsedFile) {
    let hashes: Vec<Hashes> = (0..file.entities.len()).map(|index| fingerprint(file, index)).collect();
    for (entity, hashes) in file.entities.iter_mut().zip(hashes) {
        entity.hash = hashes.full;
        entity.signature_hash = hashes.signature;
        entity.body_hash = hashes.body;
        entity.shallow_hash = hashes.shallow;
    }
}

fn fingerprint(file: &ParsedFile, index: usize) -> Hashes {
    let entity = &file.entities[index];
    let tokens = &file.tokens;
    let full = structural_hash(file, &tokens[entity.tokens.clone()], entity.depth);

    let (signature, body) = match &entity.body_tokens {
        Some(body) => {
            let header = tokens[entity.tokens.start..body.start].iter().chain(&tokens[body.end..entity.tokens.end]);
            (structural_hash(file, header, entity.depth), structural_hash(file, &tokens[body.clone()], entity.depth))
        }
        None => (full, 0),
    };

    let shallow = if entity.is_container {
        let mut own = Vec::new();
        let mut at = entity.tokens.start;
        for child in file.children_of(Some(index)) {
            let nested = &file.entities[child].tokens;
            own.extend(&tokens[at..nested.start.max(at)]);
            at = nested.end.max(at);
        }
        own.extend(&tokens[at..entity.tokens.end.max(at)]);
        structural_hash(file, own, entity.depth)
    } else {
        full
    };

    Hashes { full, signature, body, shallow }
}
