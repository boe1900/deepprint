#[derive(Debug, Clone, Copy)]
pub(crate) struct PaginationWindow {
    pub(crate) page: usize,
    pub(crate) page_size: usize,
    pub(crate) total: usize,
    pub(crate) total_pages: usize,
    pub(crate) start: usize,
}

pub(crate) fn normalize_pagination(
    page: Option<usize>,
    page_size: Option<usize>,
    total: usize,
    default_page_size: usize,
    max_page_size: usize,
) -> PaginationWindow {
    let normalized_page_size = page_size
        .unwrap_or(default_page_size)
        .clamp(1, max_page_size.max(1));
    let total_pages = if total == 0 {
        1
    } else {
        total.div_ceil(normalized_page_size)
    };
    let normalized_page = page.unwrap_or(1).max(1).min(total_pages);
    let start = (normalized_page - 1).saturating_mul(normalized_page_size);

    PaginationWindow {
        page: normalized_page,
        page_size: normalized_page_size,
        total,
        total_pages,
        start,
    }
}
