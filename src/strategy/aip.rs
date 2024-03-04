use crate::TransactionIterator;

#[derive(Debug)]
pub struct AIPError(&'static str);

impl std::fmt::Display for AIPError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "AIPError: {}", self.0)
    }
}

impl std::error::Error for AIPError {}

pub fn aip_monthly<'a>(
    mut it: TransactionIterator<'a>,
    day: u32,
    amounts: &[f64],
    fee_rates: &[f64],
) -> Result<TransactionIterator<'a>, Box<dyn std::error::Error>> {
    let total_amounts = amounts.iter().sum();
    while it.next_month(Some(day)).is_some() {
        it.inflow(total_amounts)?;
        for j in 0..it.nfunds() {
            let amount = amounts[j];
            it.buy(j, amount, amount * fee_rates[j])?;
        }
    }
    Ok(it)
}
