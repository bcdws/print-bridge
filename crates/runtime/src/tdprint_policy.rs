//! TdPrint 发行版：打印格式策略。
//!
//! 仓库只打印面单（PDF / 图片 / raw 指令），不需要 Office 文档。禁用 docx / xlsx / pptx 转换，
//! 避免允许连接的网页或远程任务触发本机 Office / WPS 自动化。所有 Office 打印（WebSocket
//! 与远程任务）都经过 `office::office_to_pdf`，在那里拦截，不会启动 Office / WPS 进程，
//! 任务以失败结束并带上 [`OFFICE_DISABLED_REASON`]。
//!
//! 定制逻辑集中在本文件，上游 office.rs 只保留调用点，便于合并上游更新。

use crate::office::OfficeConvertError;

/// Office 文档打印是否开放。
pub const OFFICE_PRINTING_ENABLED: bool = false;

/// 拒绝 Office 文档打印时返回给调用方的原因。
pub const OFFICE_DISABLED_REASON: &str =
    "TdPrint 已禁用 Office 文档（docx/xlsx/pptx）打印，请转换为 PDF 后打印";

/// Office 文档打印未开放时返回的转换错误，由 `office_to_pdf` 在转换前调用。
pub fn office_printing_guard() -> Result<(), OfficeConvertError> {
    if OFFICE_PRINTING_ENABLED {
        return Ok(());
    }
    Err(OfficeConvertError::ConverterUnavailable {
        converter: "TdPrint",
        reason: OFFICE_DISABLED_REASON.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::office::{office_to_pdf, OfficeFormat};

    #[tokio::test]
    async fn office_to_pdf_is_rejected_before_any_converter_runs() {
        let dir = std::env::temp_dir().join(format!("tdprint-policy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("a.docx");
        std::fs::write(&input, b"not really a docx").unwrap();
        let output = dir.join("a.pdf");

        let error = office_to_pdf(&input, OfficeFormat::Docx, &output)
            .await
            .unwrap_err();

        assert!(error.to_string().contains(OFFICE_DISABLED_REASON));
        assert!(!output.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
