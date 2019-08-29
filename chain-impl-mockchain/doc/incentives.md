# Incentives

## Overview

At each epoch transition, the goal is to incentivise the participants
in the system : stake pools for their operating cost and their delegators
for their individual stake contribution.

Fees are collected on the duration of an epoch, from transactions (and other
possible type of fees contributions) and deposited into a central rewards pot.

Also, as further incentives, a defined amount per epoch is sourced from the
reward escrow and contributed into the epoch rewards.

Once the reward amount is known, the treasury takes a contribution out of it
and the remaining total is splitted and distributed according to individual
pool block creation success rate. So each pool is assigned a certain share of
the total.

Once every pool shares are known, each share is further divided between the
stake pool owners (representing their operating costs and incentives to run a
public secure/working/maintained pool), and the individual contributing
stake towards this specific pool.

    ┏━━━━━━━━━━━━━┓ 
    ┃Reward Escrow┃
    ┗━━━━━━━━━━━━━┛                    ╭ Block        ╭ 
               │   ┏━━━━━━━━━━━━━┓     │ Creators     │  Stake
               ╞══>┃Epoch Rewards┃═╤═> │ For      ═╤═>│ Delegators
               │   ┗━━━━━━━━━━━━━┛ │   │ This      │  ╰ 
          Σ Fees                   │   ╰ Epoch     │  
                                   │               ╰─>─ Pool Owners
                                   │   ┏━━━━━━━━┓ 
                                   ╰─>─┃Treasury┃
                                       ┗━━━━━━━━┛ 

## Reward collection

### Reward Escrow

A fix amount of total reward is commited at genesis time to rewards
participants in the system. This is escrowed in a special account until it
has been drained completely.

At each epoch, a specific configurable amount if contributed towards the
epoch rewards. As there's only a specific known amount of value in the system
once this pot is depleted, then no contributions are made.

The usual expectations is that at start of the system, the fees collected
by usage (transactions, etc) is going to be small depending on adoption rate,
so as early incentives to contribute into the network, the initial
contribution starts at a specific value, then is reduce as time progress.

Genesis creators have full control on the specific amount and rates,
and each specific values are inscribed into block0 initial values.

There's fundamentally many potential choices for how rewards are contributed back,
and here's two potential valid examples:

* Linear formula: `calc = C - rratio * (#epoch / erate)`
* Halving formula: `calc = C * rratio ^ (#epoch / erate)`

where

* `C` is a constant representing the starting point of the contribution. note that it only gives the amount at epoch=0 in the special linear case.
* `rratio` is the reducing ratio and need to between 0.0 and 1.0.
  Further requirement is that this ratio is expressed in fractional form (e.g. 1/2),which allow calculation in integer form (see implementation details).
  Intuitively, it represent the contribution reducing factor.
* `erate` is the rate at which the contribution is reduce. e.g. erate=100 means that
  every 100 epochs, the calculation is reduce further.

And the actual contribution into the epoch reward is:

    contribution = MIN(reward_escrow, MAX(0, calc))

The escrow amount is adjusted as such:

    reward_escrow -= contribution

### Epoch Fees

This one is simply of the sum of all the usage fees usage collected since the
previous reward distribution. Typically all the block that are not empty will
contains fees related to certificates and transactions, that are just added
to the total fees collected so far at each block application.

## Reward distribution

Once the reward pot is composed, the treasury takes a cut on the total,
and then each pool get reward related by their stake in the system

    UPSCALE(x) = x * 10^9
    DOWNSCALE(x) = x / 10^9

    treasury_contribution = TREASURY_CONTRIBUTION(reward_total)
    pool_contribution = reward_total - treasury_contribution

    TREASURY += treasury_contribution

    unit_reward = UPSCALE(pool_contribution) / #blocks_epoch
    remaining = UPSCALE(pool_contribution) % #blocks_epoch

    for each pool
        pool.account = DOWNSCALE(unit_reward * #pool.blocks_created)
    
Any non null amount could be arbitrarily gifted further to the treasury, or
could be considered a bootstrap contribution toward the next epoch reward pot.

### Pool distribution

For each pool, we split each `pool.account` into a owner part and the stake reward part. Further:

    UPSCALE_STAKE(x) = x * 10^18
    DOWNSCALE_STAKE(x) = x / 10^18

    UPSCALE_OWNER(x) = x * 10^9
    DOWNSCALE_OWNER(x) = x / 10^9

    owners_contribution += OWNER_CONTRIBUTION(pool.account)
    stake_contribution = pool.account - owner_contribution

    stake_unit_reward = UPSCALE_STAKE(stake_contribution) / pool.stake
    stake_remainder = UPSCALE_STAKE(stake_contribution) % pool.stake

    owner_unit_reward = UPSCALE_OWNER(owner_contribution) / pool.owners
    owner_remainder = UPSCALE_OWNER(owner_contribution) % pool.owners

    for each owner
        owner.account += owner_unit_reward
    owner[BLOCK_DEPTH % #owners].account += owner_remainder
    for each contributor
        contributor.account += (contributor.stake * stake_unit_reward)
    contributor.

Note: The stake scaling is stronger here as the precision required is also more
important here and the values can be much smaller than the previous algorithm.

Note: We rewards an arbitrary owner of the pool with the 

## General implementation details

Every arithmetic operations are conducted on ℕ (natural numbers).

All due care has been taken so that algorithm related to coins are lossless and
implemented using fixed size unsigned integer. Overflow or underflow are 
designed to not happens, and if they occur should be a fatal error and the
result of using the wrong fixed size types.

Typically for a 64 bits total value/stake, all division/modulus/scaling operation
should be done pre-casted to 128 bits. It's possible to also sidestep this issue
by using multi precision arithmetic, although all the scaling operations
should remain the same to prevent any differences in the computed values.

Every time the integer division `/` is used, precaution should be taken to
not forget the remainder (operator `%`).

Both `OWNER_CONTRIBUTION` and `TREASURY_CONTRIBUTION` are calculation algorithm
that should return a number inferior or equal to the input. Both algorithms
can take other unspecified inputs from parameter as well if necessary, provided
that the constraint hold.

    OWNER_CONTRIBUTION : n ∈ ℕ → { r ∈ ℕ | r ≤ n }
    TREASURY_CONTRIBUTION : n ∈ ℕ → { r ∈ ℕ | r ≤ n }

For ratio scaling, we expect that the numerator is multiplied first with no overflow, then the integer division of denumerator occurs. effectively:

        A
    V * - = (V * A) / B
        B
