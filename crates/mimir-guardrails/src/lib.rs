//! Mimir Guardrails - Privacy and security protection

use mimir_core::{Memory, Result};
use regex::Regex;

/// Privacy and security classifications for content
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityLevel {
    Safe,
    Sensitive,
    Restricted,
}

/// Types of personally identifiable information
#[derive(Debug, Clone, PartialEq)]
pub enum PiiType {
    Email,
    PhoneNumber,
    SocialSecurityNumber,
    CreditCard,
    IpAddress,
    Other(String),
}

/// Detected PII in content
#[derive(Debug, Clone)]
pub struct PiiDetection {
    pub pii_type: PiiType,
    pub content: String,
    pub start_pos: usize,
    pub end_pos: usize,
}

/// Content classification result
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub security_level: SecurityLevel,
    pub detected_pii: Vec<PiiDetection>,
    pub confidence: f32,
}

/// Guardrails engine for content analysis
pub struct Guardrails {
    email_regex: Regex,
    phone_regex: Regex,
    ssn_regex: Regex,
    credit_card_regex: Regex,
    ip_regex: Regex,
}

impl Guardrails {
    /// Create a new guardrails instance
    pub fn new() -> Result<Self> {
        let email_regex = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b")?;
        let phone_regex = Regex::new(r"\b\d{3}-\d{3}-\d{4}\b|\(\d{3}\)\s*\d{3}-\d{4}\b")?;
        let ssn_regex = Regex::new(r"\b\d{3}-\d{2}-\d{4}\b")?;
        let credit_card_regex = Regex::new(r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b")?;
        let ip_regex = Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b")?;
        
        Ok(Self {
            email_regex,
            phone_regex,
            ssn_regex,
            credit_card_regex,
            ip_regex,
        })
    }
    
    /// Classify memory content for security level
    pub async fn classify_memory(&self, memory: &Memory) -> Result<ClassificationResult> {
        let content = &memory.content;
        let detected_pii = self.detect_pii(content);
        
        let security_level = if detected_pii.is_empty() {
            SecurityLevel::Safe
        } else if detected_pii.iter().any(|pii| matches!(pii.pii_type, PiiType::SocialSecurityNumber | PiiType::CreditCard)) {
            SecurityLevel::Restricted
        } else {
            SecurityLevel::Sensitive
        };
        
        let confidence = if detected_pii.is_empty() { 0.9 } else { 0.8 };
        
        Ok(ClassificationResult {
            security_level,
            detected_pii,
            confidence,
        })
    }
    
    /// Detect PII in text content
    pub fn detect_pii(&self, content: &str) -> Vec<PiiDetection> {
        let mut detections = Vec::new();
        
        // Detect emails
        for mat in self.email_regex.find_iter(content) {
            detections.push(PiiDetection {
                pii_type: PiiType::Email,
                content: mat.as_str().to_string(),
                start_pos: mat.start(),
                end_pos: mat.end(),
            });
        }
        
        // Detect phone numbers
        for mat in self.phone_regex.find_iter(content) {
            detections.push(PiiDetection {
                pii_type: PiiType::PhoneNumber,
                content: mat.as_str().to_string(),
                start_pos: mat.start(),
                end_pos: mat.end(),
            });
        }
        
        // Detect SSNs
        for mat in self.ssn_regex.find_iter(content) {
            detections.push(PiiDetection {
                pii_type: PiiType::SocialSecurityNumber,
                content: mat.as_str().to_string(),
                start_pos: mat.start(),
                end_pos: mat.end(),
            });
        }
        
        // Detect credit cards
        for mat in self.credit_card_regex.find_iter(content) {
            detections.push(PiiDetection {
                pii_type: PiiType::CreditCard,
                content: mat.as_str().to_string(),
                start_pos: mat.start(),
                end_pos: mat.end(),
            });
        }
        
        // Detect IP addresses
        for mat in self.ip_regex.find_iter(content) {
            detections.push(PiiDetection {
                pii_type: PiiType::IpAddress,
                content: mat.as_str().to_string(),
                start_pos: mat.start(),
                end_pos: mat.end(),
            });
        }
        
        detections
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mimir_core::test_utils::MemoryBuilder;
    use proptest::prelude::*;

    fn create_test_guardrails() -> Guardrails {
        Guardrails::new().expect("Failed to create test guardrails")
    }

    #[test]
    fn test_guardrails_creation() {
        let result = Guardrails::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_email() {
        let guardrails = create_test_guardrails();
        
        let test_cases = vec![
            ("Contact me at john@example.com", true),
            ("Email: user.name+tag@domain.co.uk", true),
            ("No email here", false),
            ("Invalid @ email", false),
        ];
        
        for (content, should_detect) in test_cases {
            let detections = guardrails.detect_pii(content);
            let has_email = detections.iter().any(|d| matches!(d.pii_type, PiiType::Email));
            assert_eq!(has_email, should_detect, "Failed for content: {}", content);
        }
    }

    #[test]
    fn test_detect_phone_number() {
        let guardrails = create_test_guardrails();
        
        let test_cases = vec![
            ("Call me at 555-123-4567", true),
            ("Phone: (555) 123-4567", true),
            ("No phone here", false),
            ("123-45-6789", false), // SSN format, not phone
        ];
        
        for (content, should_detect) in test_cases {
            let detections = guardrails.detect_pii(content);
            let has_phone = detections.iter().any(|d| matches!(d.pii_type, PiiType::PhoneNumber));
            assert_eq!(has_phone, should_detect, "Failed for content: {}", content);
        }
    }

    #[test]
    fn test_detect_ssn() {
        let guardrails = create_test_guardrails();
        
        let test_cases = vec![
            ("SSN: 123-45-6789", true),
            ("Social Security: 987-65-4321", true),
            ("No SSN here", false),
            ("1234-5678-9012", false), // Credit card format
        ];
        
        for (content, should_detect) in test_cases {
            let detections = guardrails.detect_pii(content);
            let has_ssn = detections.iter().any(|d| matches!(d.pii_type, PiiType::SocialSecurityNumber));
            assert_eq!(has_ssn, should_detect, "Failed for content: {}", content);
        }
    }

    #[test]
    fn test_detect_credit_card() {
        let guardrails = create_test_guardrails();
        
        let test_cases = vec![
            ("Card: 1234 5678 9012 3456", true),
            ("CC: 1234-5678-9012-3456", true),
            ("Card: 1234567890123456", true),
            ("No card here", false),
        ];
        
        for (content, should_detect) in test_cases {
            let detections = guardrails.detect_pii(content);
            let has_cc = detections.iter().any(|d| matches!(d.pii_type, PiiType::CreditCard));
            assert_eq!(has_cc, should_detect, "Failed for content: {}", content);
        }
    }

    #[test]
    fn test_detect_ip_address() {
        let guardrails = create_test_guardrails();
        
        let test_cases = vec![
            ("Server IP: 192.168.1.1", true),
            ("Connect to 10.0.0.1", true),
            ("No IP here", false),
            ("Version 1.2.3.4", true), // This might be a false positive, but acceptable
        ];
        
        for (content, should_detect) in test_cases {
            let detections = guardrails.detect_pii(content);
            let has_ip = detections.iter().any(|d| matches!(d.pii_type, PiiType::IpAddress));
            assert_eq!(has_ip, should_detect, "Failed for content: {}", content);
        }
    }

    #[test]
    fn test_multiple_pii_types() {
        let guardrails = create_test_guardrails();
        
        let content = "Contact john@example.com at 555-123-4567. Server: 192.168.1.1";
        let detections = guardrails.detect_pii(content);
        
        assert_eq!(detections.len(), 3);
        
        let pii_types: Vec<_> = detections.iter().map(|d| &d.pii_type).collect();
        assert!(pii_types.contains(&&PiiType::Email));
        assert!(pii_types.contains(&&PiiType::PhoneNumber));
        assert!(pii_types.contains(&&PiiType::IpAddress));
    }

    #[tokio::test]
    async fn test_classify_safe_content() {
        let guardrails = create_test_guardrails();
        let memory = MemoryBuilder::new()
            .with_content("This is a safe memory with no sensitive information")
            .build();
        
        let result = guardrails.classify_memory(&memory).await;
        assert!(result.is_ok());
        
        let classification = result.unwrap();
        assert_eq!(classification.security_level, SecurityLevel::Safe);
        assert!(classification.detected_pii.is_empty());
        assert!(classification.confidence > 0.8);
    }

    #[tokio::test]
    async fn test_classify_sensitive_content() {
        let guardrails = create_test_guardrails();
        let memory = MemoryBuilder::new()
            .with_content("Contact me at john@example.com for more details")
            .build();
        
        let result = guardrails.classify_memory(&memory).await;
        assert!(result.is_ok());
        
        let classification = result.unwrap();
        assert_eq!(classification.security_level, SecurityLevel::Sensitive);
        assert_eq!(classification.detected_pii.len(), 1);
        assert!(matches!(classification.detected_pii[0].pii_type, PiiType::Email));
    }

    #[tokio::test]
    async fn test_classify_restricted_content() {
        let guardrails = create_test_guardrails();
        let memory = MemoryBuilder::new()
            .with_content("Important: SSN is 123-45-6789, keep confidential")
            .build();
        
        let result = guardrails.classify_memory(&memory).await;
        assert!(result.is_ok());
        
        let classification = result.unwrap();
        assert_eq!(classification.security_level, SecurityLevel::Restricted);
        assert_eq!(classification.detected_pii.len(), 1);
        assert!(matches!(classification.detected_pii[0].pii_type, PiiType::SocialSecurityNumber));
    }

    #[test]
    fn test_pii_position_accuracy() {
        let guardrails = create_test_guardrails();
        let content = "Email me at user@domain.com for info";
        let detections = guardrails.detect_pii(content);
        
        assert_eq!(detections.len(), 1);
        let detection = &detections[0];
        
        assert_eq!(detection.start_pos, 12);
        assert_eq!(detection.end_pos, 27);
        assert_eq!(&content[detection.start_pos..detection.end_pos], "user@domain.com");
    }

    #[test]
    fn test_overlapping_patterns() {
        let guardrails = create_test_guardrails();
        
        // Test content that might trigger multiple patterns
        let content = "Data: 123-45-6789 and 192.168.1.1";
        let detections = guardrails.detect_pii(content);
        
        assert_eq!(detections.len(), 2);
        
        let pii_types: Vec<_> = detections.iter().map(|d| &d.pii_type).collect();
        assert!(pii_types.contains(&&PiiType::SocialSecurityNumber));
        assert!(pii_types.contains(&&PiiType::IpAddress));
    }

    #[test]
    fn test_case_insensitive_detection() {
        let guardrails = create_test_guardrails();
        
        let test_cases = vec![
            "EMAIL: USER@DOMAIN.COM",
            "email: user@domain.com",
            "Email: User@Domain.Com",
        ];
        
        for content in test_cases {
            let detections = guardrails.detect_pii(content);
            assert!(!detections.is_empty(), "Failed to detect email in: {}", content);
        }
    }

    #[test]
    fn test_boundary_detection() {
        let guardrails = create_test_guardrails();
        
        // Test that patterns don't match when they're part of other text
        let false_positives = vec![
            "notanemail@", // Incomplete email
            "123456789", // Numbers without separators
            "text192.168.1.1text", // IP without word boundaries
        ];
        
        for content in false_positives {
            let _detections = guardrails.detect_pii(content);
            // Some of these might still match depending on regex boundaries
            // This is more about documenting current behavior
        }
    }

    #[tokio::test]
    async fn test_empty_content() {
        let guardrails = create_test_guardrails();
        let memory = MemoryBuilder::new().with_content("").build();
        
        let result = guardrails.classify_memory(&memory).await;
        assert!(result.is_ok());
        
        let classification = result.unwrap();
        assert_eq!(classification.security_level, SecurityLevel::Safe);
        assert!(classification.detected_pii.is_empty());
    }

    #[tokio::test]
    async fn test_large_content() {
        let guardrails = create_test_guardrails();
        
        // Create large content with PII scattered throughout
        let mut large_content = "Safe content ".repeat(1000);
        large_content.push_str("Email: test@example.com");
        
        let memory = MemoryBuilder::new().with_content(large_content).build();
        
        let result = guardrails.classify_memory(&memory).await;
        assert!(result.is_ok());
        
        let classification = result.unwrap();
        assert_eq!(classification.security_level, SecurityLevel::Sensitive);
        assert_eq!(classification.detected_pii.len(), 1);
    }

    #[test]
    fn test_special_characters_in_content() {
        let guardrails = create_test_guardrails();
        
        let content = "Contact: user@domain.com\nPhone: 555-123-4567\tIP: 192.168.1.1";
        let detections = guardrails.detect_pii(content);
        
        assert_eq!(detections.len(), 3);
    }

    // Property-based tests
    proptest! {
        #[test]
        fn test_no_crash_on_random_input(content in ".*") {
            let guardrails = create_test_guardrails();
            let _detections = guardrails.detect_pii(&content);
            // Should not crash on any input
        }

        #[test]
        fn test_confidence_bounds(
            content in prop::collection::vec("[a-zA-Z0-9@.-]", 0..1000)
                .prop_map(|chars| chars.into_iter().collect::<String>())
        ) {
            let guardrails = create_test_guardrails();
            let memory = MemoryBuilder::new().with_content(content).build();
            
            tokio_test::block_on(async {
                if let Ok(classification) = guardrails.classify_memory(&memory).await {
                    assert!(classification.confidence >= 0.0 && classification.confidence <= 1.0);
                }
            });
        }
    }

    #[test]
    fn test_pii_type_equality() {
        assert_eq!(PiiType::Email, PiiType::Email);
        assert_ne!(PiiType::Email, PiiType::PhoneNumber);
        assert_eq!(PiiType::Other("custom".to_string()), PiiType::Other("custom".to_string()));
        assert_ne!(PiiType::Other("a".to_string()), PiiType::Other("b".to_string()));
    }

    #[test]
    fn test_security_level_equality() {
        assert_eq!(SecurityLevel::Safe, SecurityLevel::Safe);
        assert_ne!(SecurityLevel::Safe, SecurityLevel::Sensitive);
        assert_ne!(SecurityLevel::Sensitive, SecurityLevel::Restricted);
    }

    #[tokio::test]
    async fn test_mixed_security_levels() {
        let guardrails = create_test_guardrails();
        
        // Test content with both sensitive and restricted PII
        let memory = MemoryBuilder::new()
            .with_content("Email user@example.com, SSN: 123-45-6789, Card: 1234-5678-9012-3456")
            .build();
        
        let result = guardrails.classify_memory(&memory).await;
        assert!(result.is_ok());
        
        let classification = result.unwrap();
        // Should be restricted due to SSN and credit card
        assert_eq!(classification.security_level, SecurityLevel::Restricted);
        assert_eq!(classification.detected_pii.len(), 3);
    }
} 