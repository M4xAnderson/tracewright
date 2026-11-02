# Official cloud provider icons are never bundled

The app ships only its own generic icons. The official AWS, Azure and Google Cloud architecture icon sets are not bundled, because none of the three vendors clearly permits redistributing them inside software: the AWS terms cover diagrams, Azure allows copying "only for the permitted use", and Google publishes no license for its icon set (as of 2026-11-02, see `docs/research/stack.md`). A later feature lets users import the official icon archives they download themselves.
