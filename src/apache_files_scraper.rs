use scraper::{ElementRef, Html, Selector};
use time::{
    Date, PrimitiveDateTime, format_description::BorrowedFormatItem, macros::format_description,
};

pub fn apache_grzyby_index_iter<'a>(html: &'a Html) -> Option<impl Iterator<Item = Element<'a>>> {
    let table_selector = Selector::parse("tbody").expect("valid selector");
    let mut rows = html.select(&table_selector).next()?.child_elements();
    let mut i = 0u8;
    let mut link_idx = 255;
    let mut size_idx = 255;
    let mut date_idx = 255;
    let a = rows.next()?;
    for column in a.child_elements() {
        if i == 255 {
            break;
        }
        if !column.value().name().eq_ignore_ascii_case("th") {
            continue;
        }

        if let Some(text) = column.text().next() {
            if text.eq_ignore_ascii_case("Name") {
                link_idx = i;
            }

            if text.eq_ignore_ascii_case("Size") {
                size_idx = i;
            }

            if text.eq_ignore_ascii_case("Last modified") {
                date_idx = i;
            }
        }

        let colspan: u32 = column
            .attr("colspan")
            .and_then(|x| x.parse().ok())
            .unwrap_or(1);

        i = i.saturating_add(u8::try_from(colspan).unwrap_or(255));
    }

    Some(
        rows.filter(|x| {
            x.child_elements()
                .any(|x| x.value().name().eq_ignore_ascii_case("td"))
        })
        .map(move |x| Element {
            element: x,
            link_idx,
            size_idx,
            date_idx,
        }),
    )
}

pub struct Element<'a> {
    element: ElementRef<'a>,
    link_idx: u8,
    size_idx: u8,
    date_idx: u8,
}

impl<'a> Element<'a> {
    fn nth_column(&self, n: u8) -> Option<ElementRef<'a>> {
        if n == 255 {
            return None;
        }

        let mut i = 0;
        for column in self.element.child_elements() {
            if !column.value().name().eq_ignore_ascii_case("td") {
                continue;
            }

            if i == n {
                return Some(column);
            }

            let colspan: u32 = column
                .attr("colspan")
                .and_then(|x| x.parse().ok())
                .unwrap_or(1);

            i = i.saturating_add(u8::try_from(colspan).unwrap_or(255));
        }

        None
    }

    pub fn get_link(&self) -> Option<ElementRef<'a>> {
        self.nth_column(self.link_idx)?.child_elements().next()
    }

    pub fn get_date(&self) -> Option<PrimitiveDateTime> {
        let text = self.nth_column(self.date_idx)?.text().next()?.trim();
        const FORMAT: &'static [BorrowedFormatItem<'static>] =
            format_description!("[year]-[month]-[day] [hour]:[minute]");
        PrimitiveDateTime::parse(text, FORMAT).ok()
    }

    pub fn get_size(&self) -> Option<u64> {
        let size = self.nth_column(self.size_idx)?.text().next()?.trim();
        let split_idx = size
            .char_indices()
            .find(|x| !x.1.is_ascii_digit())
            .map(|x| x.0)
            .unwrap_or(0);

        let suffix = &size[split_idx..];
        let size: u64 = (size[..split_idx]).parse().ok()?;

        Some(match suffix {
            "" => size,
            "k" | "K" => 2u64.pow(10) * size,
            "m" | "M" => 2u64.pow(20) * size,
            "g" | "G" => 2u64.pow(30) * size,
            "p" | "P" => 2u64.pow(40) * size,
            "e" | "E" => 2u64.pow(50) * size,
            _ => return None,
        })
    }
}
