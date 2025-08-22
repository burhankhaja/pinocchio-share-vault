## Pinocchio Share Vault 

**Architecture**
- Initializer *(*anyone)* , calls initialize so that program creates a token Mint that will be used for issuing shares on deposit.
- users receive shares in 1:1 ratio for all the SOL lamports deposited
- users redeem X amount of shares for X amount of  SOL by depositing back shares into vault, where shares get burnt.
