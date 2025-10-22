//! Wrapped sqlite internals. Inspired by <https://docs.rs/rusqlite/latest/rusqlite>.

use std::{
    array,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use anyhow::{anyhow, Context as _};

use crate::bindings::hermes::sqlite::api;

/// Sqlite connection.
/// Closes on drop.
pub struct Connection(api::Sqlite);

impl Connection {
    /// Open a writable sqlite connection.
    pub fn open(in_memory: bool) -> anyhow::Result<Self> {
        api::open(false, in_memory)
            .map(Self)
            .context("Opening connection")
    }

    /// Close the connection explicitly.
    pub fn close(&self) -> anyhow::Result<()> {
        self.0.close().context("Closing connection")
    }

    /// Prepare sqlite statement.
    pub fn prepare<'a>(
        &'a self,
        sql: &str,
    ) -> anyhow::Result<Statement<'a>> {
        self.0
            .prepare(sql)
            .map(|inner| Statement(Some(inner), PhantomData))
            .context("Preparing statement")
    }

    /// Executes sqlite query without preparation.
    pub fn execute(
        &self,
        sql: &str,
    ) -> anyhow::Result<()> {
        self.0.execute(sql).context("Executing raw sql")
    }

    /// Begin a new sqlite transaction.
    ///
    /// # Note
    ///
    /// Nested transactions are not supported.
    pub fn begin(&mut self) -> anyhow::Result<Transaction<'_>> {
        self.0.execute("BEGIN").context("Beginning transaction")?;
        Ok(Transaction(self))
    }

    fn rollback(&self) -> anyhow::Result<()> {
        self.0.execute("ROLLBACK").context("Transaction rollback")
    }

    fn commit(&self) -> anyhow::Result<()> {
        self.0.execute("COMMIT").context("Committing transaction")
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Sqlite transaction.
/// Implements [`Deref`] to [`Connection`].
/// Automatically does rollback on drop.
pub struct Transaction<'conn>(&'conn mut Connection);

impl Transaction<'_> {
    /// Explicitly consume and rollback transaction.
    pub fn rollback(self) -> anyhow::Result<()> {
        self.0.rollback()
    }

    /// Consumes and commits sqlite transaction.
    pub fn commit(self) -> anyhow::Result<()> {
        self.0.commit()
    }
}

impl Deref for Transaction<'_> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Transaction<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        let _ = self.0.rollback();
    }
}

/// Sqlite statement. Automatically finalizes on [`Drop`].
pub struct Statement<'conn>(Option<api::Statement>, PhantomData<&'conn ()>);

impl Statement<'_> {
    fn finalize(&mut self) -> anyhow::Result<()> {
        self.0
            .take()
            .map_or(Ok(()), api::Statement::finalize)
            .context("Finalizing statement")
    }

    /// Binds provided parameters and executes the statement.
    ///
    /// Returns [`Rows`] that can be mapped and iterated on.
    pub fn query(
        &mut self,
        params: &[&api::Value],
    ) -> anyhow::Result<Rows<'_>> {
        self.0
            .as_mut()
            .ok_or_else(|| anyhow!("Stepped into finalized statement"))
            .and_then(|stmt| {
                params
                    .into_iter()
                    .zip(1u32..)
                    .try_for_each(|(&p, i)| stmt.bind(i, p))
                    .context("Binding query parameters")
                    .map(|()| Rows {
                        stmt: Some(stmt),
                        current: None,
                    })
            })
            .context("Executing prepared query")
    }

    /// Binds provided parameters and executes the statement.
    ///
    /// Maps the first row returned. The rest of the rows are ignored.
    pub fn query_one<T, F>(
        &mut self,
        params: &[&api::Value],
        map_f: F,
    ) -> anyhow::Result<T>
    where
        F: FnOnce(&Row<'_>) -> anyhow::Result<T>,
    {
        self.query(params)?
            .step()?
            .map_or_else(|| Err(anyhow!("Expected at least one row")), map_f)
    }

    /// Same as [`Self::query_one`], but maps using [`TryFrom`].
    /// See [`Row::values_as`].
    pub fn query_one_as<T>(
        &mut self,
        params: &[&api::Value],
    ) -> anyhow::Result<T>
    where
        T: for<'a> TryFrom<&'a Row<'a>, Error = anyhow::Error>,
    {
        self.query_one(params, |row| row.try_into())
    }
}

impl<'conn> Drop for Statement<'conn> {
    fn drop(&mut self) {
        let _ = self.finalize();
    }
}

/// Query output.
/// Automatically resets the statement on [`Drop`].
pub struct Rows<'stmt> {
    stmt: Option<&'stmt api::Statement>,
    current: Option<Row<'stmt>>,
}

impl<'stmt> Rows<'stmt> {
    fn reset(&mut self) -> anyhow::Result<()> {
        self.stmt
            .take()
            .map_or(Ok(()), api::Statement::reset)
            .context("Resetting statement")
    }

    /// Get the next row if there are any left in output.
    pub fn step(&mut self) -> anyhow::Result<Option<&Row<'stmt>>> {
        if let Some(stmt) = self.stmt {
            match stmt.step() {
                Ok(api::StepResult::Row) => {
                    self.current = Some(Row(stmt));
                    Ok(self.current.as_ref())
                },
                Ok(api::StepResult::Done) => {
                    let res = self.reset();
                    self.current = None;
                    res.map(|()| self.current.as_ref())
                },
                Err(e) => {
                    let _ = self.reset(); // prevents infinite loop on error
                    self.current = None;
                    Err(anyhow::Error::from(e))
                },
            }
        } else {
            self.current = None;
            Ok(self.current.as_ref())
        }
        .context("Statement step")
    }

    /// Map rows by closure.
    pub fn map<T, F>(
        mut self,
        mut f: F,
    ) -> impl Iterator<Item = anyhow::Result<T>> + use<'stmt, T, F>
    where
        F: FnMut(&Row<'stmt>) -> T,
    {
        std::iter::from_fn(move || self.step().transpose().map(|res| res.map(&mut f)))
    }

    /// Map step results by closure.
    pub fn and_then<T, F>(
        mut self,
        mut f: F,
    ) -> impl Iterator<Item = anyhow::Result<T>> + use<'stmt, T, F>
    where
        F: FnMut(anyhow::Result<&Row<'stmt>>) -> anyhow::Result<T>,
    {
        std::iter::from_fn(move || self.step().transpose().map(&mut f))
    }

    /// Same as [`Self::and_then`], but maps using [`TryFrom`].
    /// See [`Row::values_as`].
    pub fn map_as<T>(self) -> impl Iterator<Item = anyhow::Result<T>> + use<'stmt, T>
    where
        T: for<'a> TryFrom<&'a Row<'a>, Error = anyhow::Error>,
    {
        self.and_then(|row| row.and_then(|row| row.try_into()))
    }
}

impl Drop for Rows<'_> {
    fn drop(&mut self) {
        let _ = self.reset();
    }
}

/// Provides column access.
pub struct Row<'stmt>(&'stmt api::Statement);

impl Row<'_> {
    /// Gets column value.
    pub fn get(
        &self,
        column: u32,
    ) -> anyhow::Result<api::Value> {
        self.0.column(column).context("Decoding column")
    }

    /// Like [`Self::get`], but additionally converts the value.
    pub fn get_as<T: TryFrom<api::Value>>(
        &self,
        column: u32,
    ) -> anyhow::Result<T>
    where
        anyhow::Error: From<T::Error>,
    {
        self.get(column)
            .and_then(|v| v.try_into().map_err(anyhow::Error::from))
    }

    /// Gets values of each column with `0..N` indices.
    pub fn values_n<const N: usize>(&self) -> anyhow::Result<[api::Value; N]> {
        let mut ret = array::from_fn::<_, N, _>(|_| api::Value::Null);
        for i in 0..N as u32 {
            ret[i as usize] = self.get(i)?;
        }
        Ok(ret)
    }

    /// Gets and converts values according to [`TryFrom`] implementation.
    ///
    /// Automatically works for tuples of [`TryFrom`] from [`api::Value`] implementors.
    ///
    /// # Example
    ///
    /// ```
    /// # use shared::utils::sqlite::Row;
    /// # fn example_fn(row: &Row) -> anyhow::Result<()> {
    /// let (int, blob, opt_string) = row.values_as()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn values_as<T>(&self) -> anyhow::Result<T>
    where
        T: for<'a> TryFrom<&'a Row<'a>, Error = anyhow::Error>,
    {
        T::try_from(self)
    }
}

impl<'stmt, const N: usize> TryFrom<&'stmt Row<'stmt>> for [api::Value; N] {
    type Error = anyhow::Error;

    fn try_from(value: &'stmt Row<'stmt>) -> anyhow::Result<Self> {
        value.values_n()
    }
}

/// Defines [`TryFrom`] from [`Row`] for tuples based on [`TryFrom`] from [`api::Value`].
macro_rules! impl_tuple_try_from_row {
    () => {
        impl_tuple_try_from_row!(@);
    };
    ($first:ident $(, $remaining:ident)*) => {
        impl_tuple_try_from_row!(@ $first $(, $remaining)*);
        impl_tuple_try_from_row!($($remaining),*);
    };
    (@ $($field:ident),*) => {
        impl<'stmt, $($field,)*> TryFrom<&'stmt Row<'stmt>> for ($($field,)*)
        where
            $($field: TryFrom<api::Value>, anyhow::Error: From<$field::Error>,)*
        {
            type Error = anyhow::Error;

            #[allow(unused_mut, unused_assignments, unused_variables)]
            fn try_from(row: &'stmt Row<'stmt>) -> anyhow::Result<Self> {
                let mut idx = 0;
                $(
                    #[expect(non_snake_case)]
                    let $field = row.get_as::<$field>(idx)?;
                    idx += 1;
                )*
                Ok(($($field,)*))
            }
        }
    };
}

impl_tuple_try_from_row!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
