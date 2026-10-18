# 04 — Business models and unit economics: software products and digital content (AI-native, Vietnam-based)

Research date: 2026-10-12. I opened every cited page on this date unless a row says otherwise. Where a page was a vendor or self-reported source, the row says so. "Not found" means I searched and could not find a primary number. I did not fill those gaps with guesses. Derived numbers (my arithmetic on the cited figures) are marked **[derived]**. Modeling assumptions without a direct source are marked **[estimate]**, and the method is given.

Note on model names: the LLM pricing pages list models that are current on 2026-10-12 (for example Claude Opus 5.5, GPT-5.6, Gemini 3.8 Flash). Prices change often. Check them again before you commit to a budget.

---

## 1. Summary: 22 key numbers

| # | Key number | Confidence | Source type |
|---|---|---|---|
| 1 | **Stripe is not available to Vietnam-based businesses.** Vietnam is missing from Stripe's country list and from the Managed Payments (Stripe merchant-of-record) list of business locations. | High | Official docs |
| 2 | **Paddle (merchant of record) accepts Vietnam-based sellers** (Vietnam is not on its unsupported list). Fee: **5% + $0.50** per checkout transaction. Paddle handles tax collection and filing. Products under $10 need bespoke pricing. | High | Official vendor page |
| 3 | **Polar (merchant of record) pays out to Vietnam** through Stripe Connect Express. Fee: **5% + $0.50** (free plan) down to 3.4% + $0.30 ($400/mo plan), **+1.5% on non-US cards**, plus payout fees of $2/mo active + 0.25% + $0.25 per payout and 0.25–1% FX. | High | Official vendor page |
| 4 | **Lemon Squeezy is moving sellers to Stripe Managed Payments** (2026 update), and Stripe Managed Payments does not support Vietnam. For sellers in unsupported countries, the post suggests incorporating through Stripe Atlas. A Vietnam-only entity should not start on Lemon Squeezy. | High (fact) / Medium (impact) | Official vendor blog |
| 5 | **Gumroad (MoR since 1 Jan 2025): 10% + $0.50** on direct sales; **30%** on Discover-marketplace sales. | High | Official vendor page |
| 6 | **Indie SaaS revenue is extremely skewed.** Median MRR is about **$145–169**. About **76%** of revenue-generating indie projects earn **under $1k MRR**, and **6–10%** reach $10k+ MRR. | Medium | Self-reported / Stripe-verified aggregator data, non-zero-revenue samples (survivorship bias) |
| 7 | **Low-price SaaS retains poorly.** For ARPA under $10/mo, the **top-quartile** 12-month logo retention is **63.1%** (≈37% annual churn ≈ **3.8% per month** [derived]). Only 2.7% of these businesses exceed 100% NRR. | Medium-high | ChartMogul, 2,100+ companies, 2022 data |
| 8 | **B2B SaaS benchmark (larger companies):** median GRR **88%**, NRR **101%**, growth **26%** (2024), new CAC ratio **$2.00** of S&M per $1 of new ARR. | Medium | Benchmarkit survey, N≈563–583 |
| 9 | **Free-to-paid conversion:** self-serve freemium **3–5%** (good) / 6–8% (great). Free trial **8–12%** (good) / 15–25% (great). | Medium | Lenny's/OpenView survey, 1,000+ products, 2023 |
| 10 | **AI coding is raising app supply sharply.** New app releases rose **60% YoY** in Q1 2026 across both stores and **80%** on iOS. In April 2026 month-to-date they were up **104%**. | Medium | Appfigures via TechCrunch, 2026-04-18 |
| 11 | **App-store commissions:** Apple Small Business Program **15%** (under $1M in proceeds). Google Play outside US/EEA/UK/JP/AU is **15% on the first $1M**. In the US/EEA/UK, Google Play subscriptions are **10% + 5% billing fee**. Shopify App Store is **0% on the first $1M** of lifetime gross (from 2025-01-01), then 15%, plus a **2.9% processing fee**. | High | Official docs |
| 12 | **Google Search referrals to publishers fell 40.2% YoY** (July 2025 to July 2026), after a 21.9% drop the year before. Search's share of publisher pageviews went from about 9% to about 5% in two years. | Medium-high | Chartbeat network (news-heavy), via SEJ |
| 13 | **68% of US Google searches ended without a click** in Jan–Apr 2026 (60.5% in 2024). AI Mode received only 0.34% of searches. | Medium | Similarweb panel via SparkToro |
| 14 | **When an AI summary appears, users click a result 8% of the time vs 15% without one.** They click a link inside the summary 1% of the time. | High (method) | Pew, 900 US adults, March 2025 |
| 15 | **AI Overviews correlate with a 58% lower CTR for the #1 result** (December 2025 vs December 2023). | Medium | Ahrefs, 300k keywords (vendor) |
| 16 | **Google's spam policies target AI content at scale.** "Scaled content abuse" means many pages generated mainly to manipulate rankings. Google says generating many pages with AI without adding value "may violate" this policy. | High | Google Search Central (updated 2026-08-28 / 2026-10-01) |
| 17 | **YouTube demonetizes "inauthentic" (mass-produced or repetitive) content** (renamed 2025-07-15). It explicitly lists "AI-generated content made with generic or unoriginal templates." | High | YouTube Help |
| 18 | **Vietnam YouTube creator RPM is ≈ $0.17 per 1,000 views, versus $5.25 in the US** (about 31x lower). | Low-medium | Third-party aggregator, sample undisclosed |
| 19 | **Vietnam has 85.6M internet users (84.2%)** and 79.0M social-media identities (end of 2025). About **70% of online ad revenue goes to cross-border platforms** (Meta, Google, TikTok). | Medium-high / Medium | DataReportal / Vietnam Advertising Association via Báo Đầu tư |
| 20 | **Vietnam backend developer median salary:** **39.9M VND/month** (5–8 years' experience) and **54.9M VND** (over 8 years). That is ≈ $1,530 and ≈ $2,110 at 26,000 VND/USD [derived]. Content-writer listings mostly show **9–15M VND/month** (≈ $350–580). | Medium | ITviec survey, N=1,839 / TopCV listings |
| 21 | **LLM API prices per 1M input/output tokens:** Claude Opus 5.5 $4/$20, Sonnet 5.5 $2/$10, Haiku 5.5 $0.10/$0.50. GPT-5.6 Sol $5/$30, Luna $0.20/$1.20. Gemini 3.8 Flash $0.75/$3.75. **Batch is −50%.** Cache reads cost about 5–10% of base input. | High | Official pricing pages |
| 22 | **Agents are token-hungry.** Agents use about **4x** the tokens of chat, and multi-agent systems about **15x**. Claude Code averages **about $13 per developer per active day** ($150–250 per month). | High (vendor) | Anthropic engineering blog / Claude Code docs |

---

## 2. Software products

### 2.1 Outcome distributions, retention, conversion

| Metric | Value | Source URL | Date | Sample / method | Limitations |
|---|---|---|---|---|---|
| Median MRR of indie projects | $169/mo. Top 25% earn over ~$800, top 10% over $10,000, top 1% over $98,500 | https://www.indiehackers.com/post/i-analyzed-5-079-stripe-verified-startups-f0f6bd053f | 2026-03-12 | 5,079 projects from the TrustMRR API (Stripe-verified revenue) | Community post. Methodology not published. Projects listed on TrustMRR self-select, so failures are under-represented. |
| Indie SaaS MRR distribution | Under $1k: 76.4%. $1k–10k: 17.5%. $10k+: 6.1%. Median: $145 | https://bigideasdb.com/state-of-indie-saas-revenue-2026 | 2026-07-03 | 3,787 businesses with non-zero MRR (from 8,000+ tracked) | **Self-reported**, zero-revenue projects excluded, survivorship bias, and the publisher sells into this market. Including zero-revenue projects would push the true distribution lower. |
| Time to $1M ARR | Median ≈ 2 years 9 months. Best-in-class ≈ 9 months. Only 13% reach $10M ARR within 10 years. | https://chartmogul.com/reports/saas-growth-report/ | 2023-08-30 | 2,200+ ChartMogul customers, 12 months to June 2023 | Single-vendor data and older. Covers companies that already use billing analytics. |
| Median ARR growth | ~30%/yr (2–2.5%/mo). Top quartile 60–70%/yr | same | 2023-08-30 | same | Same |
| Retention, ARPA under $10/mo | Top-quartile NRR 65.1%. Top-quartile logo retention 63.1%. Only 5.3% have GRR over 85%. | https://chartmogul.com/reports/saas-retention-report/ | 2023-11-01 | 2,100+ companies, calendar 2022 | Top quartile, not median, so the median business does worse. Monthly churn ≈ 3.8% [derived from 63.1%]. |
| Retention, ARPA over $500/mo | Top-quartile NRR 109.3%. Logo retention 85.7% | same | same | same | Same |
| Median time to first revenue | **Not found** in a primary source | — | — | — | — |
| B2B SaaS (mostly $1M+ ARR) | Growth 26%, NRR 101%, GRR 88%, new CAC ratio $2.00 (top-quartile figure in report: $2.82) | https://www.hibob.com/wp-content/uploads/2025-SaaS-Performance-Metrics-Benchmarks.pdf | 2025 report, CY2024 data | Benchmarkit survey, N=563–583 | Survey of larger B2B companies, not micro-SaaS. CAC payback values appear only in a chart. |
| Private B2B SaaS NRR, $25–50k ACV | Median 102% (quartiles 97–111%). Median growth across companies over $1M ARR: 24% | https://www.saas-capital.com/research/saas-retention-benchmarks-for-private-b2b-companies/ | 2025-09-18 | SaaS Capital 2025 survey | Sample size not on the page. Small-ACV values appear only in chart images. |
| AI-native vs AI-enabled growth | Under $1M ARR: 100% vs 75% YoY. Gross margin 75% vs 80% | https://highalpha.com/blog/does-it-pay-to-be-ai-native-growth-rates-for-ai-companies | 2025-12-12 | 800+ survey responses | Mostly VC-backed companies, and AI-native status is self-classified. Shows AI inference lowers gross margin by about 5 points. |
| Subscription churn (Recurly network) | SaaS 3.22% (voluntary 2.16%, involuntary 1.06%). Digital media 4.14% | https://www.recurly.com/research/churn-rate-benchmarks | July 2026 data | Recurly network, size not stated | **The page is internally inconsistent.** It labels 3.22% "annual" in one place and gives 3.04% elsewhere. These values look like monthly churn. Use with caution. Involuntary churn (failed payments) is about one third of the total. |
| Freemium to paid | Self-serve 3–5% good, 6–8% great. Sales-assisted 5–7% / 10–15% | https://www.lennysnewsletter.com/p/what-is-a-good-free-to-paid-conversion | 2023-08-01 | 1,000+ products, mostly B2B | Self-reported survey, older. |
| Free trial to paid | 8–12% good, 15–25% great | same | same | same | Does not separate card-required (opt-out) from no-card (opt-in) trials. |
| Opt-out vs opt-in trial | 48.8% vs 18.2% | https://www.crazyegg.com/blog/free-to-paid-conversion-rate/ (citing FirstPageSage, 86 companies) | n.d. | Secondary citation | **Secondary source.** The metric definition is unclear, and the article's table contradicts its text. Low confidence. |

### 2.2 Customer acquisition and distribution platforms

| Metric | Value | Source URL | Date | Sample / method | Limitations |
|---|---|---|---|---|---|
| CAC payback (B2B SaaS) | No clean value found. Benchmarkit shows payback by ACV only in a chart. The new CAC ratio of $2.00 per $1 of new ARR implies ≈ 24 months / gross margin at 100% margin [derived] | Benchmarkit PDF above | 2025 | — | **CAC benchmarks for self-serve micro-SaaS were not found** in primary data. |
| Shopify App Store | 0% revenue share on the first $1M of lifetime gross since 2025-01-01, then 15%. $19 one-time registration. **2.9% processing fee** on all billing | https://shopify.dev/docs/apps/launch/distribution/revenue-share | Accessed 2026-10-12 | Official | Developers with over $20M in prior-year App Store earnings pay 15% on everything. |
| Apple App Store | 15% under the Small Business Program (under $1M in proceeds). 10% for subscriptions after the first year under EU alternative terms | https://developer.apple.com/app-store/small-business-program/ | Accessed 2026-10-12 | Official | The standard rate (30%) is not stated on this page. |
| Google Play | Outside AU/EEA/JP/UK/US: 15% on the first $1M, then 30%, and 15% on subscriptions. In US/EEA/UK (from 2026-06-30) and AU/JP (from 2026-09-30): subscriptions 10% + 5% billing fee. Other transactions: 20–25% + 5% | https://support.google.com/googleplay/android-developer/answer/112622 | Accessed 2026-10-12 | Official | The new structure is being rolled out region by region. |
| Chrome Web Store | Built-in payments ended 2021-02-01. Developers must run their own billing (for example Paddle or Polar) | https://developer.chrome.com/docs/webstore/cws-payments-deprecation | Accessed 2026-10-12 | Official | The store has no revenue share, but it also does not help with monetization. |
| WordPress.org plugin directory | **Not researched in depth.** The directory is free and has no marketplace billing. | — | — | — | — |

### 2.3 Payments for a Vietnam-based seller selling globally

| Option | Vietnam seller? | Fee | MoR / tax | Source URL | Notes |
|---|---|---|---|---|---|
| Stripe (direct) | **No.** Vietnam is not on the global availability list. | 2.9% + $0.30 US, +1.5% international, +1% FX | No (Stripe Tax is separate) | https://stripe.com/global ; https://stripe.com/pricing | Only possible with a foreign entity (for example Singapore, or US via Atlas). |
| Stripe Managed Payments | **No.** Supported business locations are US, CA, EU/EEA, CH, GB, AU, HK, JP, SG. | +3.5% on top of Stripe processing (≈ 6.4% + $0.30 domestic US [derived]) | Yes, 80+ countries | https://docs.stripe.com/payments/managed-payments/eligibility | Fully automated digital products only. Human-delivered services are excluded. |
| Paddle | **Yes** (not on the unsupported list) | 5% + $0.50. Products under $10 need custom pricing | Yes: collects, files and remits sales tax/VAT | https://www.paddle.com/pricing ; https://www.paddle.com/help/start/intro-to-paddle/which-countries-are-supported-by-paddle | Payout fees are not stated on the pricing page. Paddle may request more information before releasing payments. |
| Polar | **Yes** (Vietnam is on the payout list via Stripe Connect Express) | 5% + $0.50 (free) to 3.4% + $0.30 ($400/mo). +1.5% non-US cards. Payout: $2/mo + 0.25% + $0.25. FX 0.25–1% | Yes | https://polar.sh/resources/pricing ; https://polar.sh/docs/merchant-of-record/supported-countries | Effective cost on a $20 non-US sale ≈ 9% before payout fees [derived]. |
| Lemon Squeezy | Vietnam bank payouts are listed today, **but** sellers are being migrated to Stripe Managed Payments, which excludes Vietnam | 5% + $0.50 | Yes | https://www.lemonsqueezy.com/pricing ; https://www.lemonsqueezy.com/blog/2026-update ; https://docs.lemonsqueezy.com/help/getting-started/supported-countries | New sign-ups are waitlisted. Platform risk is high. |
| Gumroad | Not verified for Vietnam specifically | 10% + $0.50 direct. 30% via Discover | Yes, since 2025-01-01 | https://gumroad.com/pricing | Expensive, but it supplies some discovery traffic. |
| Domestic (Vietnam): MoMo | Yes | Merchant collection fee 0%. Withdrawal free up to 30M VND/mo, then 0.5% | n/a | https://www.momo.vn/tin-tuc/thong-bao/quan-trong-cap-nhat-cac-chinh-sach-phi-va-quyen-4904 | Policy dated 2023-09-20. Card and online gateway fees not stated. |
| Domestic: VNPay | — | **Not found** (no public merchant rate card located) | — | — | Gateway fees are usually negotiated. |

**What MoR means in practice:** the MoR is the legal seller to the end customer. It calculates, collects and remits VAT, GST and US sales tax, and it handles fraud and chargebacks. The Vietnam company then receives a payout from the MoR (a B2B export of services), not direct consumer revenue. Vietnamese tax treatment of these payouts is not researched here.

### 2.4 Effect of AI coding on supply and price pressure

| Metric | Value | Source URL | Date | Sample / method | Limitations |
|---|---|---|---|---|---|
| New mobile app releases | +60% YoY in Q1 2026 (both stores), +80% on iOS. April 2026 month-to-date +104% (both) and +89% (iOS). Productivity and utilities moved into the top 5 categories | https://techcrunch.com/2026/04/18/the-app-store-is-booming-again-and-ai-may-be-why/ | 2026-04-18 | Appfigures estimates | Attributing the rise to AI is plausible but not proven. No absolute counts are given. |
| Price pressure or revenue per app | **Not found.** I found no primary dataset on falling prices or revenue per app caused by AI-built competitors. | — | — | — | Treat as a qualitative risk. Model it as higher CAC and higher churn. |

---

## 3. Digital content

### 3.1 Google Search and AI answers

| Metric | Value | Source URL | Date | Sample / method | Limitations |
|---|---|---|---|---|---|
| Scaled content abuse policy | "Many pages are generated for the primary purpose of manipulating search rankings and not helping users" | https://developers.google.com/search/docs/essentials/spam-policies | Updated 2026-08-28 | Official policy | Enforcement thresholds are not public. |
| Site reputation policy | Third-party content published on a host "mainly because of that host's already-established ranking signals". Reviewers check the host's editorial oversight | same | same | Official | This rules out "parasite SEO" as a distribution strategy. |
| Generative-AI content guidance | AI content is allowed, but mass generation without added value may violate the scaled-content policy. Google recommends fact-checking and considering disclosure. Merchant Center requires the IPTC `TrainedAlgorithmicMedia` tag on AI images | https://developers.google.com/search/docs/fundamentals/using-gen-ai-content | Updated 2026-10-01 | Official | — |
| CTR with an AI summary | 8% vs 15% without. 1% click links inside the summary. Sessions end after the results page 26% vs 16% | https://www.pewresearch.org/short-reads/2025/07/22/google-users-are-less-likely-to-click-on-links-when-an-ai-summary-appears-in-the-results/ | 2025-07-22 (March 2025 data) | 900 US adults, browsing panel, 68,879 searches | Correlational and US only. AI summaries appeared on 18% of searches at the time, and coverage has grown since. |
| #1 result CTR with an AI Overview | −58% (0.073 → 0.016, December 2023 to December 2025) | https://ahrefs.com/blog/ai-overviews-reduce-clicks-update/ | 2026-02-04 | 300k keywords, GSC desktop data | **Vendor**, desktop only, modelled baseline. The page cites Seer at −49% to −65%. |
| Zero-click searches (US) | 68.01% in Jan–Apr 2026 (60.45% in 2024, ~49% in 2019). AI Mode received 0.34% of searches | https://sparktoro.com/blog/in-2026-less-than-one-third-of-google-searches-still-send-a-click/ | June 2026 | Similarweb clickstream panel | Panels differ across years, and the Google app is excluded, so the real figure is probably higher. |
| Google Search referrals to publishers | −40.2% YoY (Jul 2025 → Jul 2026), after −21.9% the year before. Search share of pageviews ~9% → 5%. Discover −34.3%. AI-chatbot referrals ≈ 0.01% of traffic | https://www.searchenginejournal.com/google-search-decline-publishers-chartbeat/590681/ | 2026-09-24 | Chartbeat client network, ~31B pageviews per month | News-heavy and site count not disclosed. Secondary report of Chartbeat's 2026 Publisher Playbook (the primary source was not opened). |

### 3.2 Newsletters, YouTube and social platforms

| Metric | Value | Source URL | Date | Sample / method | Limitations |
|---|---|---|---|---|---|
| Substack free → paid | "5–10% of free subscribers convert… 10% being a rate to aim for" | https://substack.com/going-paid-guide | Accessed 2026-10-12 | **Vendor claim** | Probably reflects writers who already have an audience and turned on paid. No method given. |
| Substack fee | 10% + Stripe 2.9% + $0.30 + 0.7% Billing | https://support.substack.com/hc/en-us/articles/360037607131-How-much-does-Substack-cost | Accessed | Official | Requires Stripe, which is **not available in Vietnam**. |
| Email open / click rates | Median open 43.46%, click 2.09%, CTOR 6.81%, unsubscribe 0.22% | https://www.mailerlite.com/blog/compare-your-email-performance-metrics-industry-benchmarks | Dec 2024 – Nov 2025 | 3.6M campaigns, 181k accounts | Apple Mail Privacy Protection inflates opens. **Vendor** data. |
| Newsletter sponsorship CPM | $15–35 consumer or under 5k subscribers. $20–50 for 5k–50k. $50–100+ niche B2B. Flat rates: $50–250 under 5k, $500–3,000 for 5k–50k | https://www.beehiiv.com/blog/newsletter-sponsorship-cost | 2025-11-09 | **Vendor blog**, partly citing third parties | No systematic dataset behind it. |
| beehiiv Ad Network | Paid plans only. Earns on CPM of unique opens or CPC (examples: $5 CPM, $2 CPC). The blog says earning starts at 2,500 subscribers | https://www.beehiiv.com/support/article/17525818225431-getting-started-with-the-ad-network | Accessed | Official | The $5/$2 figures are illustrative. beehiiv's take rate is not stated. |
| YouTube "inauthentic content" | Renamed from "repetitious content" on 2025-07-15. Covers "mass-produced, generic, repetitive". Explicitly lists "AI-generated content made with generic or unoriginal templates" | https://support.google.com/youtube/answer/1311392 | Updated 2025-07-15 | Official | YouTube describes this as a clarification of an existing rule. |
| YouTube AI disclosure | Required for realistic altered or synthetic content. Not required for AI-written scripts, titles or thumbnails. YouTube can also auto-label via C2PA or detection | https://support.google.com/youtube/answer/14328491 | Accessed | Official | — |
| Meta "AI info" label | Applied when industry AI signals are detected or the uploader self-discloses. Content is labelled rather than removed | https://about.fb.com/news/2024/04/metas-approach-to-labeling-ai-generated-content-and-manipulated-media/ | 2024 (updated Sep 2024) | Official | Penalties for not disclosing are not described. |
| TikTok AI labelling | **Not verified** (the help page did not render) | — | — | — | Check manually. |

### 3.3 Display ads and affiliate

| Metric | Value | Source URL | Date | Sample / method | Limitations |
|---|---|---|---|---|---|
| Mediavine | Full program requires $5,000 in annual ad revenue. Journey requires 1,000 Tier-1 sessions in 30 days (PPC Land reports 10,000) | https://www.mediavine.com/mediavine-requirements/ | Accessed | Official | Revenue share and RPM not published. |
| Raptive | 25,000 monthly pageviews (down from 100k on 2025-10-16). Sites under 100k pageviews need 50% Tier-1 traffic (US/UK/CA/AU/NZ) | https://ppc.land/is-your-site-finally-ready-the-new-math-behind-premium-ad-network-approvals/ | 2026-01-03 | Secondary report of Raptive's page (raptive.com failed to load) | **Vietnamese-language traffic will not qualify** because of the Tier-1 geography rule. |
| Display RPM (US lifestyle sites) | Raptive example: $17.26 and a $20 cohort average (2021). No current network-wide average is published | https://raptive.com/blog/proven-rpm-lifts/ | 2021-03-30 | **Vendor** single example | Old and selective. |
| Vietnam vs US YouTube RPM | VN $0.17, US $5.25 per 1,000 views. VN playback CPM $1.11 | https://dynamoi.com/data/youtube-adsense-rpm/vn | Data to 2026-10-07 | Aggregated YouTube Analytics, sample undisclosed | **Third-party, low transparency.** Direction consistent with the general market; magnitude uncertain. |
| AdSense web RPM for Vietnamese traffic | **No reliable primary source found** | — | — | — | Use the YouTube ratio as a proxy only. |
| Amazon Associates (US) | 10% luxury beauty. 4.5% books and kitchen. 4% "all other". 3% home, toys, sports. 2.5% PC. 1% grocery and health | https://affiliate-program.amazon.com/help/node/topic/GRXPHT8U84RAYDXZ | Accessed (no effective date shown) | Official | Rates were cut in 2017 and 2020. No 2025–26 change verified. |

### 3.4 Vietnam market

| Metric | Value | Source URL | Date | Sample / method | Limitations |
|---|---|---|---|---|---|
| Population / internet users | 102M / 85.6M (84.2%) | https://datareportal.com/reports/digital-2026-vietnam | End of 2025 | Kepios (ITU, GSMA etc.) | Estimates. |
| Social media identities | 79.0M (77.6%). Ad reach: Facebook 79.0M, YouTube 62.1M, TikTok 76.1M (18+) | same | same | Platform ad tools | Ad reach is not the same as active users. The TikTok figure exceeds 100% of adults. |
| Total / online ad market | ~$2.3B total and ~$1.21B online (2023, Statista). Online forecast for 2024: $3–4B (Vietnam Advertising Association). **About 70% goes to cross-border platforms** | https://baodautu.vn/dua-quang-cao-tren-mang-xa-hoi-vao-khuon-kho-d223056.html | 2024-08-26 | Secondary press report | The two estimates conflict, which shows the market size is poorly measured. |
| Cloud enterprise application software market | $860.5M (2024), forecast $2.26B in 2033 | https://www.imarcgroup.com/report/vi/vietnam-cloud-enterprise-application-software-market | 2025-10-10 | Paid market-research report | Methodology not visible. The stated CAGR does not reconcile with the endpoints. Low confidence. |
| Vietnamese SME willingness to pay for SaaS | **Not found** (no primary survey) | — | — | — | Gap: needs primary customer interviews. |
| Backend developer median salary | <1 yr 12.4M, 1–2 yrs 25.35M, 3–4 yrs 30.1M, 5–8 yrs 39.9M, >8 yrs 54.9M VND/mo. Tech lead 51.8M | https://itviec.com/report/vietnam-it-salary-and-recruitment-market | Fieldwork May–Nov 2025 | Online survey, N=1,839 | Self-selected ITviec users. Gross pay. Employer social-insurance costs not included (not researched). |
| IT department head salary | Median 53.6M VND/mo (≈ $2,040) | https://en.baobacninhtv.vn/salaries-continue-to-rise-in-vietnam-for-senior-it-roles-postid436985.bbg | 2026-01-11 | TopCV survey, 3,000+ respondents, Q3 2025 | Press summary. |
| Content writer / editor pay | Listings: content writer 9–15M, editor 12–20M, brand copywriter 16–22M VND/mo. 114 open listings | https://www.topcv.vn/tim-viec-lam-content-writer | 2026-10-11 | Live job listings | Only about 8 listings visible, so this is not a statistic. |
| USD/VND | SBV reference rate 25,058, market ceiling ~26,310 (2025-06-24). **This doc uses 26,000 as the modelling rate** | https://theinvestor.vn/usd-price-in-vietnam-peaks-topping-vnd26300-d16108.html | 2025-06-24 | Press | Current-day rate not retrieved. Recheck before use. |

---

## 4. AI operating costs

### 4.1 LLM APIs (USD per 1M tokens, standard tier)

| Provider / model | Input | Cached input | Output | Batch | Source URL | Date |
|---|---|---|---|---|---|---|
| Anthropic Claude Opus 5.5 | $4 | $0.20 (5-minute cache write $5) | $20 | 50% off ($2/$10) | https://platform.claude.com/docs/en/about-claude/pricing | Accessed 2026-10-12 |
| Claude Sonnet 5.5 | $2 | $0.10 | $10 | $1/$5 | same | same |
| Claude Haiku 5.5 (prompts ≤100K tokens) | $0.10 | $0.01 | $0.50 | $0.05/$0.25 | same | same |
| Claude Fable / Mythos 5.1 (top tier) | $10 | $0.25 | $50 | $5/$25 | same | same |
| Claude web search tool | $10 per 1,000 searches, plus tokens | | | | same | same |
| OpenAI GPT-5.6 Sol (flagship) | $5 | $0.50 | $30 | −50% | https://openai.com/api/pricing/ | same |
| GPT-5.6 Terra (mid-tier) | $2 | $0.20 | $12 | −50% | same | same |
| GPT-5.6 Luna (small) | $0.20 | $0.02 | $1.20 | −50% | same | same |
| Google Gemini 3.1 Pro preview (≤200k) | $2 | $0.20 | $12 | $1/$6 | https://ai.google.dev/gemini-api/docs/pricing | same. Many Google rates are listed "through 2026-12-31" and **double on 2027-01-01** |
| Gemini 3.8 Flash | $0.75 | $0.075 | $3.75 | $0.375/$1.875 | same | same |
| Gemini 3.1 Flash-Lite | $0.25 | $0.025 | $1.50 | $0.125/$0.75 | same | same |

### 4.2 Media generation

| Item | Price | Source URL | Notes |
|---|---|---|---|
| Veo 3.1 Standard / Fast / Lite (per second, with audio) | $0.40 / $0.10–0.12 / $0.05–0.08 (720p–1080p). 4k: $0.60 / $0.30 | https://ai.google.dev/gemini-api/docs/pricing | A 60-second clip costs ≈ $3 (Lite) / $6–7 (Fast) / $24 (Standard) [derived], before failed takes. |
| Runway Gen-4.5 | 12 credits per second ≈ $0.10–0.23/s depending on plan (Standard $12/mo for 625 credits up to Max $76/mo for 9,500, billed annually) | https://runway.com/pricing | Assumes the full credit allotment is used. |
| OpenAI GPT-Image-2 | Image tokens: $8 in / $30 out per 1M | https://openai.com/api/pricing/ | Per-image token counts were not stated, so a per-image $ figure is **not derived**. |
| Sora API pricing | **Not listed** on the OpenAI pricing page | — | — |
| Gemini 3.8 Flash TTS | $0.50 in / $9 out (audio) per 1M tokens ≈ $0.00225 per 10 s ≈ **$0.0135/min** [derived] | https://ai.google.dev/gemini-api/docs/pricing | — |
| ElevenLabs API TTS | Multilingual v2 $0.08, Flash/Turbo $0.04 per 1K characters (about 1 minute) | https://elevenlabs.io/pricing/api | The tier-to-row mapping on the page is inferred. |

### 4.3 Tokens per task

| Task | Evidence | Cost estimate |
|---|---|---|
| Agent vs chat multiplier | Agents use about 4x the tokens of chat. Multi-agent systems use about 15x. Token usage explains 80% of performance variance on BrowseComp — https://www.anthropic.com/engineering/multi-agent-research-system | — |
| Coding: one dev-day of agentic coding | Average ≈ $13 per developer per active day, $150–250 per month, and under $30/day for 90% of users. A sample session shows $0.55 on Sonnet 4.6 (1.2k input, 5.3k output, 940k cache read) — https://code.claude.com/docs/en/costs | A **small coding change** costs about **$0.30–2** on Sonnet-class models [estimate, anchored on the $0.55 sample session]. Multi-step features cost $5–30. |
| 1,500-word article with research | **No primary measurement found.** | **[estimate] Method:** 8 web searches ($0.08) + about 10 pages read (~80k input) + 3 drafting/review passes (~90k input) + ~15k output including thinking. That gives ≈ 170k in / 15k out. **Haiku 5.5 ≈ $0.10. Sonnet 5.5 ≈ $0.57** (≈ $0.40 with caching). **Opus 5.5 ≈ $1.06.** A multi-agent research setup at 5–15x costs ≈ $3–8. |

### 4.4 Tooling (list prices accessed 2026-10-12)

| Tool | Price | Source URL |
|---|---|---|
| Vercel | Hobby free (non-commercial only). Pro $20 per developer seat per month with $20 usage credit. Functions: $0.128 per active CPU-hour, $0.60 per 1M invocations | https://vercel.com/pricing |
| Cloudflare Workers Paid | $5/mo minimum: 10M requests and 30M CPU-ms included, then $0.30 per 1M requests. No egress fees. R2 storage $0.015/GB-month with free egress. D1 includes 5 GB | https://developers.cloudflare.com/workers/platform/pricing/ |
| Fly.io | shared-cpu-1x $2.19/mo (256MB) or $6.70 (1GB). Egress $0.02/GB (NA/EU), $0.04/GB (APAC). Managed Postgres from $38/mo | https://docs.fly.io/about/pricing |
| Resend | Free: 3k emails/mo (100/day). Pro $20 for 50k. Scale $90 for 100k. Overage $0.90 per 1k | https://resend.com/pricing |
| Postmark | $15–18/mo for 10k emails. Overage $1.20–1.80 per 1k | https://postmarkapp.com/pricing |
| Amazon SES | $0.10 per 1k emails (à la carte). Dedicated IP $24.95/mo | https://aws.amazon.com/ses/pricing/ |
| Sentry | Developer free (5k errors). Team $26/mo (annual, 50k errors). Business $80/mo | https://sentry.io/pricing/ |
| beehiiv | Free up to 2,500 subscribers. Lite $49/mo. Pro $95/mo (annual billing). 0% take on paid subscriptions; Stripe fees still apply | https://www.beehiiv.com/pricing |
| Kit | Free up to 10k subscribers. Creator $33/mo. Pro $66/mo. Paid products: 3.5% + $0.30 processing + 0.6% Kit fee | https://kit.com/pricing |
| Substack | Free to publish. 10% of paid revenue + Stripe fees (Stripe is unavailable in Vietnam) | see §3.2 |

---

## 5. Inputs for a cost/profit model

Units: USD per month unless stated. "Src" points to the tables above. **[E]** = estimate, with the reasoning given.

### 5.1 Micro-SaaS / software product (global, English, sold through a MoR)

| Parameter | Low | Base | High | Basis |
|---|---|---|---|---|
| Price per customer (ARPA) | $9 | $19 | $49 | [E] typical self-serve tiers. ChartMogul shows retention improves sharply as ARPA rises (§2.1). |
| Payment / MoR cost as % of revenue | 6.0% ($49, Paddle) | 7.6% ($19: 5% + $0.50 = $1.45) | 10.6% ($9, Paddle, which needs custom pricing under $10) up to ~12% (Polar + intl + payout) | Paddle and Polar pricing (§2.3) [derived] |
| Monthly logo churn | 3% | 5% | 8% | ChartMogul top-quartile retention for ARPA under $10 implies 3.8%/mo, and the median business is worse (§2.1). Recurly SaaS ~3.2% is ambiguous. **[E]** base 5% for a new, low-ARPA product. |
| Visitor/signup → paid conversion | 2% (freemium) | 8% (no-card trial) | 15% (trial, strong fit) | Lenny's/OpenView (§2.1) |
| Probability a given product reaches $1k MRR | ~10% | ~20% | ~25% | About 24% of **non-zero-revenue** indie projects exceed $1k MRR (BigIdeasDB). The top 25% exceed ~$800 (TrustMRR). Including zero-revenue projects lowers the share, so the base is set below 24% [E]. |
| Probability of reaching $10k MRR | 2% | 5% | 10% | 6.1% (BigIdeasDB) and 10% (TrustMRR) of revenue-generating projects. |
| Median MRR of a launched product after ~1–2 yrs | $0 | $150 | $800 | Medians $145–169 (§2.1). |
| Gross margin after AI inference, hosting and MoR | 65% | 75% | 85% | High Alpha AI-native median 75% (§2.1) [E] for MoR adjustment. |
| Hosting and tooling per product (early stage) | $5 (CF Workers) | $50 (Vercel Pro + Resend + Sentry) | $150 (+ Fly Postgres, Postmark) | §4.4 |
| App-store share (if distributed via a store) | 0% (Shopify, first $1M) + 2.9% | 15% (Apple SBP / Play) | 30% | §2.2 |
| AI coding cost to build an MVP | $50 | $300 | $1,500 | [E] Claude Code $13 per active day × 5–100 agent-days (§4.3). |
| Ongoing AI maintenance and coding per product | $20 | $100 | $400 | [E] 2–30 small changes per month at $0.3–2 each, plus occasional features. |
| CAC (paid acquisition) | $20 | $80 | $250 | **[E] No primary micro-SaaS CAC data found.** Benchmarkit's new CAC ratio of $2 per $1 of ARR implies CAC ≈ 24× MRR per customer (for ARPA $19 that is about $456) for larger B2B. Treat the paid channel as uneconomic below roughly $30 ARPA unless organic or marketplace distribution works. |

### 5.2 Digital content (articles, newsletter, video)

| Parameter | Low | Base | High | Basis |
|---|---|---|---|---|
| AI cost per 1,500-word researched article | $0.10 (Haiku-class) | $0.60 (Sonnet-class) | $3–8 (multi-agent / Opus) | §4.3 [E] method |
| Human editor review per article (Vietnam) | 0.25 h | 0.5 h | 1.5 h | [E]. Google recommends human fact-checking (§3.1). At 15M VND/mo ≈ $577 / 176 h ≈ **$3.3/h** [derived]. |
| Vietnam content editor salary | 9M VND ($346) | 15M VND ($577) | 22M VND ($846) | TopCV listings (§3.4) |
| Vietnam senior engineer (5–8 yrs) | 30M VND ($1,160) | 39.9M VND ($1,535) | 54.9M VND ($2,110) | ITviec medians (§3.4), gross pay. Employer insurance not included. |
| YoY change in organic Google traffic (planning) | −10% | −25% | −45% | Chartbeat −40% (news), zero-click 68%, AI Overviews −58% CTR on #1 (§3.1) [E] range. |
| Display RPM per 1,000 pageviews, Tier-1 English traffic | $5 | $15 | $25 | [E]. Only vendor anecdotes ($17–20, Raptive 2021). Premium networks need 25k pageviews/mo and 50% Tier-1 traffic. |
| Display RPM, Vietnamese traffic | $0.10 | $0.30 | $1.00 | [E] proxy: YouTube VN RPM $0.17 vs US $5.25 (31x gap) (§3.3). Low confidence. |
| YouTube RPM (VN / US audience) | $0.13 / — | $0.17 / $5.25 | — | dynamoi (§3.3). Low transparency. |
| Newsletter free → paid | 1% | 3% | 10% | Substack vendor claim of 5–10% for writers who turned on paid [E]: new AI-run newsletters without a personal brand should be modelled lower. |
| Newsletter opens / clicks | 30% / 1% | 40% / 2% | 50% / 3% | MailerLite medians 43.5% / 2.1% (opens inflated by MPP) |
| Sponsorship CPM (per 1,000 opens/subscribers) | $15 | $30 | $60 | beehiiv blog ranges (vendor) |
| Newsletter platform cost at 10k subscribers | $0 (Kit free) | $49 (beehiiv Lite) | $95 | §4.4 |
| AI video cost per finished minute | $3 (Veo Lite) | $10 (Veo Fast + retakes) | $50 (Veo Standard + retakes) | §4.2 [E] retake multiplier of 1.5–2x |
| TTS per minute | $0.014 (Gemini Flash TTS) | $0.04 (ElevenLabs Flash) | $0.08 (ElevenLabs Multilingual) | §4.2 |
| Amazon affiliate commission | 1% | 3–4% | 10% | Amazon rate card (§3.3) |

### 5.3 Shared and company-level

| Parameter | Low | Base | High | Basis |
|---|---|---|---|---|
| LLM blended price (agentic work, $ per 1M tokens with caching) | $0.30 (Haiku/Flash-Lite heavy) | $2 (Sonnet-class) | $6 (Opus-class) | §4.1 [E] blend assuming 70–90% cache hits |
| Agentic token multiplier vs single prompt | 4x | 8x | 15x | Anthropic (§4.3) |
| Batch discount for non-urgent jobs | 0% | 50% on eligible share | 50% | All three providers |
| Google price increase risk | — | — | 2x on 2027-01-01 for many Gemini rates | Gemini pricing page note |
| FX rate | 25,000 | 26,000 | 27,000 VND/USD | §3.4 (reference only) |

### 5.4 Key gaps (not found in primary sources)
- Time to first revenue for indie SaaS.
- CAC for self-serve micro-SaaS.
- Quantified price or revenue pressure from AI-built competitors.
- Web AdSense RPM for Vietnamese-language traffic.
- VNPay merchant fees.
- Vietnamese SME willingness to pay for SaaS.
- TikTok AI-label rules (page did not render).
- Sora API price.
- Per-image API price.
- Employer statutory payroll costs in Vietnam.
- Vietnamese tax treatment of MoR payouts.

These need targeted follow-up or primary interviews before the base case is relied on.
