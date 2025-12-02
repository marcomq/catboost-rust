use catboost_rust::{CatBoostError, Model, ObjectsOrderFeatures};

/// The binary content of a minimal, valid CatBoost model.
/// This model was trained on 3 float features and 1 categorical feature.
const TEST_MODEL_BYTES: &[u8] = include_bytes!("test_model.bin");

#[test]
fn test_load_model_from_file() -> Result<(), CatBoostError> {
    // Test that loading a non-existent model file returns an error.
    let result = Model::load("nonexistent_model.bin");
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_load_model_from_buffer() -> Result<(), CatBoostError> {
    let model = Model::load_buffer(TEST_MODEL_BYTES)?;
    assert_eq!(model.get_tree_count(), 1000);
    Ok(())
}

#[test]
fn test_get_model_metadata() -> Result<(), CatBoostError> {
    let model = Model::load_buffer(TEST_MODEL_BYTES)?;
    assert_eq!(model.get_dimensions_count(), 1);
    assert_eq!(model.get_tree_count(), 1000);
    assert_eq!(
        model.get_float_features_count(),
        3,
        "Incorrect float feature count"
    );
    assert_eq!(model.get_cat_features_count(), 1);
    Ok(())
}

#[test]
fn test_predict_float_only() -> Result<(), CatBoostError> {
    let model = Model::load_buffer(TEST_MODEL_BYTES)?;

    let float_features = vec![vec![1.0, 2.0, 3.0]];
    let cat_features: Vec<Vec<String>> = vec![vec!["".to_string()]];

    let predictions = model.calc_model_prediction(float_features, cat_features)?;
    assert_eq!(predictions.len(), 1);
    assert!(
        (predictions[0] - 0.3184967798337403).abs() < 1e-9,
        "Prediction value was: {}",
        predictions[0]
    );
    Ok(())
}

#[test]
fn test_predict_float_and_cat() -> Result<(), CatBoostError> {
    let model = Model::load_buffer(TEST_MODEL_BYTES)?;

    let float_features = vec![vec![1.0, 2.0, 3.0]];
    let cat_features = vec![vec!["a".to_string()]];

    let predictions = model.calc_model_prediction(float_features, cat_features)?;
    assert_eq!(predictions.len(), 1);
    assert!(
        (predictions[0] - 0.3184967798337403).abs() < 1e-9,
        "Prediction value was: {}",
        predictions[0]
    );
    Ok(())
}

#[test]
fn test_predict_batch() -> Result<(), CatBoostError> {
    let model = Model::load_buffer(TEST_MODEL_BYTES)?;

    let float_features = vec![vec![1.0, 2.0, 3.0], vec![5.0, 4.0, 3.0]];
    let cat_features = vec![
        vec!["a".to_string()], // This will be hashed
        vec!["d".to_string()],
    ];

    let predictions = model.calc_model_prediction(float_features, cat_features)?;
    assert_eq!(predictions.len(), 2);
    assert!(
        (predictions[0] - 0.3184967798337403).abs() < 1e-9,
        "Prediction value for first item was: {}",
        predictions[0]
    );
    assert!(
        (predictions[1] - 0.3184967798337403).abs() < 1e-9,
        "Prediction value for second item was: {}",
        predictions[1]
    );
    Ok(())
}

#[test]
fn test_predict_with_objects_order_features() -> Result<(), CatBoostError> {
    let model = Model::load_buffer(TEST_MODEL_BYTES)?;

    let features = ObjectsOrderFeatures::new()
        .with_float_features(&[&[1.0f32, 2.0, 3.0]])
        .with_cat_features(&[&["a" as &str]]);

    let predictions = model.predict(features)?;
    assert_eq!(predictions.len(), 1);
    assert!(
        (predictions[0] - 0.3184967798337403).abs() < 1e-9,
        "Prediction value was: {}",
        predictions[0]
    );
    Ok(())
}

#[test]
fn test_predict_with_incorrect_float_feature_count() {
    let model = Model::load_buffer(TEST_MODEL_BYTES).unwrap();

    // Model expects 3 float features, we provide 2
    let float_features = vec![vec![1.0, 2.0]];
    let cat_features: Vec<Vec<String>> = vec![vec![]];

    let result = model.calc_model_prediction(float_features, cat_features);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error
            .description
            .contains("insufficient float features vector size")
    );
}

#[test]
fn test_predict_with_incorrect_cat_feature_count() {
    let model = Model::load_buffer(TEST_MODEL_BYTES).unwrap();

    let float_features = vec![vec![1.0, 2.0, 3.0]];
    // Model expects 1 cat feature, we provide 2
    let cat_features = vec![vec!["a".to_string(), "b".to_string()]];

    let result = model.calc_model_prediction(float_features, cat_features);
    assert!(result.is_ok()); // The C++ library ignores extra/missing cat features with a warning
}

#[test]
#[cfg(catboost_feature_indices)]
fn test_get_feature_names() -> Result<(), CatBoostError> {
    let model = Model::load_buffer(TEST_MODEL_BYTES)?;

    // Note: The dummy model does not contain feature names.
    // This test just ensures the functions can be called without error
    // and return the correct feature names for this specific model.
    let expected_names: Vec<String> = vec![
        "0".to_string(),
        "1".to_string(),
        "wind direction".to_string(),
        "3".to_string(),
    ];
    assert_eq!(model.get_feature_names()?, expected_names);

    let expected_float_names: Vec<String> = vec!["0".to_string(), "1".to_string(), "3".to_string()];
    assert_eq!(model.get_float_feature_names()?, expected_float_names);

    let expected_cat_names: Vec<String> = vec!["wind direction".to_string()];
    assert_eq!(model.get_cat_feature_names()?, expected_cat_names);

    assert!(model.get_text_feature_names()?.is_empty());
    assert!(model.get_embedding_feature_names()?.is_empty());

    Ok(())
}
