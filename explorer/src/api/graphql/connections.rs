use async_graphql::{FieldResult, OutputType, SimpleObject};
use std::convert::TryFrom;

#[derive(SimpleObject)]
pub struct ConnectionFields<C: OutputType + Send + Sync> {
    pub total_count: C,
}

pub struct ValidatedPaginationArguments<I> {
    pub first: Option<usize>,
    pub last: Option<usize>,
    pub before: Option<I>,
    pub after: Option<I>,
}

pub struct PageMeta {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub total_count: u64,
}

fn compute_range_boundaries(
    total_elements: InclusivePaginationInterval<u64>,
    pagination_arguments: ValidatedPaginationArguments<u64>,
) -> PaginationInterval<u64>
where
{
    use std::cmp::{max, min};

    let InclusivePaginationInterval {
        upper_bound,
        lower_bound,
    } = total_elements;

    // Compute the required range of blocks in two variables: [from, to]
    // Both ends are inclusive
    let mut from: u64 = match pagination_arguments.after {
        Some(cursor) => max(cursor + 1, lower_bound),
        // If `after` is not set, start from the beginning
        None => lower_bound,
    };

    let mut to: u64 = match pagination_arguments.before {
        Some(cursor) => {
            if cursor == 0 {
                return PaginationInterval::Empty;
            }
            min(cursor - 1, upper_bound)
        }
        // If `before` is not set, start from the beginning
        None => upper_bound,
    };

    // Move `to` enough values to make the result have `first` blocks
    if let Some(first) = pagination_arguments.first {
        to = min(
            from.checked_add(u64::try_from(first).unwrap())
                .and_then(|n| n.checked_sub(1))
                .unwrap_or(to),
            to,
        );
    }

    // Move `from` enough values to make the result have `last` blocks
    if let Some(last) = pagination_arguments.last {
        from = max(
            to.checked_sub(u64::try_from(last).unwrap())
                .and_then(|n| n.checked_add(1))
                .unwrap_or(from),
            from,
        );
    }

    PaginationInterval::Inclusive(InclusivePaginationInterval {
        lower_bound: from,
        upper_bound: to,
    })
}

pub fn compute_interval<I>(
    bounds: PaginationInterval<I>,
    pagination_arguments: ValidatedPaginationArguments<I>,
) -> FieldResult<(PaginationInterval<I>, PageMeta)>
where
    I: TryFrom<u64> + Clone,
    u64: From<I>,
{
    let pagination_arguments = pagination_arguments.cursors_into::<u64>();
    let bounds = bounds.bounds_into::<u64>();

    let (page_interval, has_next_page, has_previous_page, total_count) = match bounds {
        PaginationInterval::Empty => (PaginationInterval::Empty, false, false, 0u64),
        PaginationInterval::Inclusive(total_elements) => {
            let InclusivePaginationInterval {
                upper_bound,
                lower_bound,
            } = total_elements;

            let page = compute_range_boundaries(total_elements, pagination_arguments);

            let (has_previous_page, has_next_page) = match &page {
                PaginationInterval::Empty => (false, false),
                PaginationInterval::Inclusive(page) => (
                    page.lower_bound > lower_bound,
                    page.upper_bound < upper_bound,
                ),
            };

            let total_count = upper_bound
                .checked_add(1)
                .unwrap()
                .checked_sub(lower_bound)
                .expect("upper_bound should be >= than lower_bound");
            (page, has_next_page, has_previous_page, total_count)
        }
    };

    Ok(page_interval
        .bounds_try_into::<I>()
        .map(|interval| {
            (
                interval,
                PageMeta {
                    has_next_page,
                    has_previous_page,
                    total_count,
                },
            )
        })
        .map_err(|_| "computed page interval is outside pagination boundaries")
        .unwrap())
}

impl<I> ValidatedPaginationArguments<I> {
    fn cursors_into<T>(self) -> ValidatedPaginationArguments<T>
    where
        T: From<I>,
    {
        ValidatedPaginationArguments {
            after: self.after.map(T::from),
            before: self.before.map(T::from),
            first: self.first,
            last: self.last,
        }
    }
}

pub enum PaginationInterval<I> {
    Empty,
    Inclusive(InclusivePaginationInterval<I>),
}

pub struct InclusivePaginationInterval<I> {
    pub lower_bound: I,
    pub upper_bound: I,
}

impl<I> PaginationInterval<I> {
    fn bounds_into<T>(self) -> PaginationInterval<T>
    where
        T: From<I>,
    {
        match self {
            Self::Empty => PaginationInterval::<T>::Empty,
            Self::Inclusive(interval) => {
                PaginationInterval::<T>::Inclusive(InclusivePaginationInterval::<T> {
                    lower_bound: T::from(interval.lower_bound),
                    upper_bound: T::from(interval.upper_bound),
                })
            }
        }
    }

    fn bounds_try_into<T>(self) -> Result<PaginationInterval<T>, <T as TryFrom<I>>::Error>
    where
        T: TryFrom<I>,
    {
        match self {
            Self::Empty => Ok(PaginationInterval::<T>::Empty),
            Self::Inclusive(interval) => Ok(PaginationInterval::<T>::Inclusive(
                InclusivePaginationInterval::<T> {
                    lower_bound: T::try_from(interval.lower_bound)?,
                    upper_bound: T::try_from(interval.upper_bound)?,
                },
            )),
        }
    }
}
