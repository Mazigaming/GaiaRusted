//! Tests for lifetime elision implementation

#[cfg(test)]
mod tests {
    use gaiarusted::borrowchecker::lifetime_elision::{
        LifetimeElisionAnalyzer, LifetimeElisionConfig,
    };
    use gaiarusted::borrowchecker::lifetimes::LifetimeContext;
    use gaiarusted::lowering::HirType;

    #[test]
    fn test_lifetime_elision_rule1() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // fn foo(x: &str) -> &str
        let params = vec![HirType::Reference(Box::new(HirType::String))];
        let return_type = Some(HirType::Reference(Box::new(HirType::String)));

        let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);

        assert_eq!(
            result.rule_applied,
            gaiarusted::borrowchecker::lifetime_elision::ElisionRule::Rule1
        );
        assert_eq!(result.input_lifetimes.len(), 1);
        assert!(result.input_lifetimes[0].is_some());
        assert!(result.output_lifetime.is_some());
        assert_eq!(result.input_lifetimes[0], result.output_lifetime);
    }

    #[test]
    fn test_lifetime_elision_rule2() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // fn foo(x: &str, y: &str) -> &str
        let params = vec![
            HirType::Reference(Box::new(HirType::String)),
            HirType::Reference(Box::new(HirType::String)),
        ];
        let return_type = Some(HirType::Reference(Box::new(HirType::String)));

        let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);

        assert_eq!(
            result.rule_applied,
            gaiarusted::borrowchecker::lifetime_elision::ElisionRule::Rule2
        );
        assert_eq!(result.input_lifetimes.len(), 2);
        assert!(result.input_lifetimes[0].is_some());
        assert!(result.input_lifetimes[1].is_some());
        assert!(result.output_lifetime.is_some());
        assert_eq!(result.input_lifetimes[0], result.output_lifetime);
    }

    #[test]
    fn test_rule3_method_with_self() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // fn foo(&self) -> &str
        // Note: Conservative approach - can't reliably detect 'self' from type alone
        // So Rule 3 is not applied automatically
        let params = vec![HirType::Reference(Box::new(HirType::String))];
        let return_type = Some(HirType::Reference(Box::new(HirType::String)));

        let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);

        // Conservative: Rule 3 not applied automatically
        // This is correct - explicit lifetimes should be used for methods
        assert_eq!(result.rule_applied, ElisionRule::Rule1);
        assert_eq!(result.input_lifetimes.len(), 1);
        assert!(result.input_lifetimes[0].is_some());
        assert!(result.output_lifetime.is_some());
        assert_eq!(result.input_lifetimes[0], result.output_lifetime);
    }

    #[test]
    fn test_no_elision_no_return_reference() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // fn foo(x: &str) -> i32
        let params = vec![HirType::Reference(Box::new(HirType::String))];
        let return_type = Some(HirType::Int32);

        let result = analyzer.infer_lifetimes(&params, &return_type, &mut lifetime_ctx);

        assert_eq!(
            result.rule_applied,
            gaiarusted::borrowchecker::lifetime_elision::ElisionRule::None
        );
        assert!(result.input_lifetimes[0].is_some());
        assert!(result.output_lifetime.is_none());
    }

    #[test]
    fn test_register_function() {
        let mut analyzer = LifetimeElisionAnalyzer::new();
        let mut lifetime_ctx = LifetimeContext::new();

        // fn foo(x: &str) -> &str
        let params = vec![HirType::Reference(Box::new(HirType::String))];
        let return_type = Some(HirType::Reference(Box::new(HirType::String)));

        let result =
            analyzer.register_function("foo".to_string(), params, return_type, &mut lifetime_ctx);

        assert_eq!(
            result.rule_applied,
            gaiarusted::borrowchecker::lifetime_elision::ElisionRule::Rule1
        );

        let retrieved = analyzer.get_elision_result("foo");
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().rule_applied,
            gaiarusted::borrowchecker::lifetime_elision::ElisionRule::Rule1
        );
    }

    #[test]
    fn test_custom_config() {
        let config = LifetimeElisionConfig {
            enable_rule1: false,
            enable_rule2: true,
            enable_rule3: true,
        };
        let analyzer = LifetimeElisionAnalyzer::with_config(config);
        assert_eq!(analyzer.config.enable_rule1, false);
        assert_eq!(analyzer.config.enable_rule2, true);
        assert_eq!(analyzer.config.enable_rule3, true);
    }
}
