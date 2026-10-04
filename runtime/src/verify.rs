/*
 * Copyright 2026 Rein contributors
 * SPDX-License-Identifier: Apache-2.0
 */

use rein_core::ArtifactRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::Path;

pub use rein_core::VerificationStatus;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct FixedPlan {
    version: String,
    expected: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct FixedVerifier {
    expected: Option<Vec<u8>>,
}

impl FixedVerifier {
    pub fn expected_file(path: impl AsRef<Path>) -> Self {
        Self {
            expected: fs::read(path).ok(),
        }
    }

    pub fn expected_bytes(bytes: &[u8]) -> Self {
        Self {
            expected: Some(bytes.to_vec()),
        }
    }

    pub fn from_expected(expected: Option<Vec<u8>>) -> Self {
        Self { expected }
    }

    pub fn expected(&self) -> Option<&[u8]> {
        self.expected.as_deref()
    }

    pub fn check(&self, bytes: &[u8]) -> VerificationStatus {
        match self.expected.as_deref() {
            Some(expected) if expected == bytes => VerificationStatus::Passed,
            Some(_) => VerificationStatus::Failed,
            None => VerificationStatus::Undetermined,
        }
    }

    pub fn plan_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&FixedPlan {
            version: "fixed-bytes-v1".into(),
            expected: self.expected.clone(),
        })
    }

    pub fn from_plan_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let plan: FixedPlan = serde_json::from_slice(bytes)?;
        if plan.version != "fixed-bytes-v1" {
            return Err("unsupported verification plan version".into());
        }
        Ok(Self::from_expected(plan.expected))
    }

    pub fn receipt(
        &self,
        plan_ref: &ArtifactRef,
        final_ref: &ArtifactRef,
        final_bytes: &[u8],
    ) -> Result<VerificationReceipt, Box<dyn Error>> {
        validate_bytes(final_ref, final_bytes)?;
        Ok(VerificationReceipt {
            version: "fixed-bytes-v1".into(),
            plan_ref: plan_ref.clone(),
            final_ref: final_ref.clone(),
            expected_hash: self.expected.as_deref().map(hash_hex),
            final_hash: hash_hex(final_bytes),
            status: self.check(final_bytes),
        })
    }

    pub fn validate_receipt(
        &self,
        receipt: &VerificationReceipt,
        plan_ref: &ArtifactRef,
        final_ref: &ArtifactRef,
        final_bytes: &[u8],
    ) -> Result<(), Box<dyn Error>> {
        let expected = self.receipt(plan_ref, final_ref, final_bytes)?;
        if receipt != &expected {
            return Err("verification receipt mismatch".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationReceipt {
    pub version: String,
    pub plan_ref: ArtifactRef,
    pub final_ref: ArtifactRef,
    pub expected_hash: Option<String>,
    pub final_hash: String,
    pub status: VerificationStatus,
}

fn validate_bytes(reference: &ArtifactRef, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    if reference.len != bytes.len() as u64 || reference.hash != hash_hex(bytes) {
        return Err("artifact bytes do not match reference".into());
    }
    Ok(())
}

fn hash_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(bytes: &[u8]) -> ArtifactRef {
        ArtifactRef {
            hash: hash_hex(bytes),
            len: bytes.len() as u64,
        }
    }

    #[test]
    fn checks_passed_failed_and_undetermined() {
        assert_eq!(
            FixedVerifier::expected_bytes(b"ok").check(b"ok"),
            VerificationStatus::Passed
        );
        assert_eq!(
            FixedVerifier::expected_bytes(b"ok").check(b"no"),
            VerificationStatus::Failed
        );
        assert_eq!(
            FixedVerifier::from_expected(None).check(b"ok"),
            VerificationStatus::Undetermined
        );
    }

    #[test]
    fn plan_roundtrips_and_rejects_unknown_version() {
        let verifier = FixedVerifier::expected_bytes(b"ok");
        let restored = FixedVerifier::from_plan_bytes(&verifier.plan_bytes().unwrap()).unwrap();
        assert_eq!(restored.expected(), Some(b"ok".as_slice()));
        assert!(FixedVerifier::from_plan_bytes(br#"{"version":"other","expected":null}"#).is_err());
    }

    #[test]
    fn receipt_validates_and_rejects_tampering() {
        let verifier = FixedVerifier::expected_bytes(b"ok");
        let plan_ref = reference(b"plan");
        let final_ref = reference(b"ok");
        let receipt = verifier.receipt(&plan_ref, &final_ref, b"ok").unwrap();
        verifier
            .validate_receipt(&receipt, &plan_ref, &final_ref, b"ok")
            .unwrap();
        let mut changed = receipt.clone();
        changed.status = VerificationStatus::Failed;
        assert!(verifier
            .validate_receipt(&changed, &plan_ref, &final_ref, b"ok")
            .is_err());
        changed = receipt.clone();
        changed.final_ref = reference(b"other");
        assert!(verifier
            .validate_receipt(&changed, &plan_ref, &final_ref, b"ok")
            .is_err());
        changed = receipt;
        changed.version = "other".into();
        assert!(verifier
            .validate_receipt(&changed, &plan_ref, &final_ref, b"ok")
            .is_err());
    }

    #[test]
    fn rejects_wrong_final_hash_or_length() {
        let verifier = FixedVerifier::expected_bytes(b"ok");
        let plan_ref = reference(b"plan");
        assert!(verifier
            .receipt(&plan_ref, &reference(b"bad"), b"ok")
            .is_err());
        assert!(verifier
            .receipt(
                &plan_ref,
                &ArtifactRef {
                    hash: hash_hex(b"ok"),
                    len: 9
                },
                b"ok"
            )
            .is_err());
    }

    #[test]
    fn none_plan_produces_undetermined_receipt() {
        let verifier = FixedVerifier::from_expected(None);
        let receipt = verifier
            .receipt(&reference(b"plan"), &reference(b"ok"), b"ok")
            .unwrap();
        assert_eq!(receipt.status, VerificationStatus::Undetermined);
        assert_eq!(receipt.expected_hash, None);
    }

    #[test]
    fn expected_hash_is_raw_bytes_sha256() {
        let verifier = FixedVerifier::expected_bytes(b"ok");
        let receipt = verifier
            .receipt(&reference(b"plan"), &reference(b"ok"), b"ok")
            .unwrap();
        assert_eq!(
            receipt.expected_hash.as_deref(),
            Some(hash_hex(b"ok").as_str())
        );
    }
}
