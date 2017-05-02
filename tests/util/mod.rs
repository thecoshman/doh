use doh::util::{RaiiGuard, TAB_SPACING, TAB_WIDTH};


#[test]
fn tab_spacing() {
    assert_eq!(TAB_SPACING.len(), TAB_WIDTH);
    for c in TAB_SPACING.chars() {
        assert_eq!(c, ' ');
    }
}

#[test]
fn raii_guard() {
    static mut END: bool = false;

    unsafe {
        END = false;
    }
    let mut start = false;
    {
        let _write = RaiiGuard::new(|| start = true, || unsafe {
            END = true;
        });
        assert_eq!(start, true);
        assert_eq!(unsafe { END }, false);
    }
    assert_eq!(start, true);
    assert_eq!(unsafe { END }, true);
}
