
# Concurrency Mechanisms to be implemented

## Serial Transactions

Serial transactions work by the following:
1. create a transaction and acquire global lock on schema
2. perform work (read, write what not) & store all changes on local "merge"
3. if commit, then merge onto main (assuming all constraints workout) and unlock
4. if rollback, unlock and discard

**Benefits:**
- There are no conflicts possible. All transactions happen one after the other.

**Drawbacks:**
- 1 transaction can only happen at a time even if they modify unrelated tables & rows.
- A transaction which takes forever will starve other transactions from doing any work.

## Single Writer with global table lock

Single writer transactions work based on the following:
1. create a transaction
2. perform any reads (acquire read lock on the database and release it)
3. the moment you perform a write, acquire a write lock on the database
4. on rollback, discard the changes and unlock
5. on commit perform the following:
    1. check constraints, if not pass then abort and require user to rollback changes
    2. if all is good then merge and then unlock

**Benefits:**
- Multiple transactions can read from the database at the same time.

**Drawbacks:**
- Writers who edit unrelated data entries will have to wait for each other to complete.
- Consider scenerio with 2 transactions T1 and T2. T1 reads, T2 reads, T1 writes, then must commit, then T2 writes. The operation written by T2 whether its an update or delete or insert will be based on the value that T1 wrote not what T2 read. Depending on the behavior required by the application, maybe someone would want to have it abort instead since this is a dirty write.

## OCC over MVCC

OCC over MVCC is implemented as follows:
1. transaction begins and acquire a pointer to the current head commit of the database as the base commit for this transaction
2. construct a shim which acts as a staged write region between this and the base commit
3. any reads made will first read the shim and then fall through
4. any writes made will find the values in the base commit and perform a write on the shim
5. on rollback discard the shim, pointer to base commit, and transaction
6. on commit do the following:
    1. acquire write lock on the upstream commit map
    2. find the diff of the upstream to the base and the diff of our shim and the base and ensure they're consistent
    3. if not consistent (operations in shim would lead to conflict like rows changed) then abort
    4. else construct a commit based on this and run constraint validation on this
    5. if constraints pass then make this the new upstream and unlock
    6. else then abort

**Benefits:**
- Multiple transactions can read and write at the same time, commits are restricted to one at a time.

**Drawbacks:**
- Transactions with conflicting values on commit are aborted and thus may require to be retried.

Some things to note:
- if upstream has changed a table (row update or table change) that we've mutated via DDL then we must abort
- if upstream has altered the DDL in a way which our row updates cannot function OR the rows that we're updating with DML have been changed, then abort.