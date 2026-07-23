"""Archive of production-ineligible placeholder values found by the mock-data audit.

This module is intentionally not imported by the Vue/Tauri runtime.  The
application is TypeScript and Rust; its live data comes from the local Axum
service and SQLite.  Keeping the retired values here makes the removal
auditable without creating a Python runtime dependency or reintroducing them
into production.
"""

FAKE_DATA_BACKUP = {
    "MOCK_LOCAL_RECEIVE_FOLDER": {
        "business_meaning": "A previous frontend startup default that exposed a specific developer Windows path before persisted settings loaded.",
        "value": r"C:\Users\fengqiao\Downloads\LYNQO",
    },
    "LEGACY_MOBILE_DEVICE_NAME_PLACEHOLDERS": {
        "business_meaning": "Retired page-local browser names that were not derived from the actual device model or stable device identity.",
        "value": {
            "IOS_PHONE": "iPhone",
            "IOS_TABLET": "iPad",
            "ANDROID_GENERIC": "Android Device",
            "WEB_GENERIC": "Mobile Browser",
        },
    },
    "LEGACY_UPDATE_SUCCESS_MESSAGE": {
        "business_meaning": "Retired About-page alert that claimed the application was current without performing an update check.",
        "value": "当前已是最新版本",
    },
    "LEGACY_TRANSIENT_ID_RANDOM_SOURCE": {
        "business_meaning": "Retired Math.random-based local temporary-ID implementation; it did not represent business data.",
        "value": "Math.random().toString(36).slice(2)",
    },
}
