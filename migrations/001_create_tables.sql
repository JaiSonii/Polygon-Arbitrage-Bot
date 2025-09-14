-- Create arbitrage opportunities table
CREATE TABLE IF NOT EXISTS arbitrage_opportunities (
    id UUID PRIMARY KEY,
    token0_address VARCHAR(42) NOT NULL,
    token1_address VARCHAR(42) NOT NULL,
    token0_symbol VARCHAR(10) NOT NULL,
    token1_symbol VARCHAR(10) NOT NULL,
    buy_dex VARCHAR(50) NOT NULL,
    sell_dex VARCHAR(50) NOT NULL,
    buy_price DECIMAL(36, 18) NOT NULL,
    sell_price DECIMAL(36, 18) NOT NULL,
    price_difference DECIMAL(36, 18) NOT NULL,
    price_difference_percentage DECIMAL(10, 4) NOT NULL,
    estimated_profit DECIMAL(36, 18) NOT NULL,
    trade_amount DECIMAL(36, 18) NOT NULL,
    gas_cost DECIMAL(36, 18) NOT NULL,
    net_profit DECIMAL(36, 18) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create price quotes table for historical data
CREATE TABLE IF NOT EXISTS price_quotes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dex_name VARCHAR(50) NOT NULL,
    token0_address VARCHAR(42) NOT NULL,
    token1_address VARCHAR(42) NOT NULL,
    token0_symbol VARCHAR(10) NOT NULL,
    token1_symbol VARCHAR(10) NOT NULL,
    price DECIMAL(36, 18) NOT NULL,
    liquidity DECIMAL(36, 18),
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_arbitrage_opportunities_timestamp ON arbitrage_opportunities(timestamp);
CREATE INDEX IF NOT EXISTS idx_arbitrage_opportunities_tokens ON arbitrage_opportunities(token0_address, token1_address);
CREATE INDEX IF NOT EXISTS idx_price_quotes_timestamp ON price_quotes(timestamp);
CREATE INDEX IF NOT EXISTS idx_price_quotes_dex_tokens ON price_quotes(dex_name, token0_address, token1_address);
