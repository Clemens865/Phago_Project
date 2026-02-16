//! Transfer (Horizontal Gene Transfer) tests.

use phago_agents::digester::Digester;
use phago_core::agent::Agent;
use phago_core::types::*;

#[test]
fn vocabulary_export_roundtrip() {
    let mut d = Digester::new(Position::new(0.0, 0.0));
    d.digest_text("cell membrane protein transport biology molecular".to_string());

    let exported = d
        .export_vocabulary()
        .expect("should have vocabulary to export");
    let cap: VocabularyCapability = serde_json::from_slice(&exported).expect("should deserialize");

    assert!(!cap.terms.is_empty(), "exported terms should not be empty");
    assert_eq!(cap.origin, d.id());
    // Terms should include keywords from digestion
    assert!(
        cap.terms.iter().any(|t| t == "cell"),
        "should contain 'cell'"
    );
}

#[test]
fn vocabulary_integrate_boosts_keywords() {
    // Create two digesters: one produces vocabulary, other integrates it
    let mut producer = Digester::new(Position::new(0.0, 0.0));
    producer.digest_text("mitochondria powerhouse oxidative phosphorylation energy".to_string());

    let vocab_bytes = producer.export_vocabulary().expect("should export");

    let mut consumer = Digester::new(Position::new(5.0, 0.0));
    let integrated = consumer.integrate_vocabulary(&vocab_bytes);
    assert!(integrated, "first integration should succeed");

    // Now digest text that includes a boosted term
    let fragments = consumer.digest_text(
        "The mitochondria produces energy through chemical reactions in biology cells".to_string(),
    );
    // "mitochondria" and "energy" should be boosted (rank higher)
    assert!(
        fragments.contains(&"mitochondria".to_string()),
        "boosted term should appear"
    );
}

#[test]
fn double_integrate_rejected() {
    let mut producer = Digester::new(Position::new(0.0, 0.0));
    producer.digest_text("cell membrane protein transport molecular".to_string());

    let vocab_bytes = producer.export_vocabulary().expect("should export");

    let mut consumer = Digester::new(Position::new(5.0, 0.0));
    assert!(
        consumer.integrate_vocabulary(&vocab_bytes),
        "first integration succeeds"
    );
    assert!(
        !consumer.integrate_vocabulary(&vocab_bytes),
        "second integration from same source rejected"
    );
}
