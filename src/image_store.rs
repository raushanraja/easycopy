use crate::store::images::ImageStore as StoreImageStore;

// ================================================================
//  IMAGE STORE (delegates to store::images)
// ================================================================
// Thin wrapper for backward compatibility. The real implementation
// lives in store/images.rs.

pub type ImageStore = StoreImageStore;
