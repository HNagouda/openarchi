# Data sources

Which open services can feed the analyses openarchi does not draw yet, checked on 2026-10-24 with real requests from this Mac around two test sites: the Westerkerk in Amsterdam (lon 4.8840, lat 52.3745, roughly RD x 120700, y 487600) and the TU/e campus in Eindhoven (lon 5.4908, lat 51.4483). Every latency below is one measured `curl` call, not an average.

The short version: almost everything is available without a key. PDOK, RIVM, LIWO, the Klimaateffectatlas, KNMI and PVGIS are open government services with no rate limits we could hit. OpenStreetMap through Overpass covers points of interest, transit stops and the footpath network, but the main Overpass server is slow and sometimes full. Open-Meteo is fast and convenient but its free tier is non-commercial only. Zoning under the Omgevingswet needs a free API key, though the older zoning plans are open through PDOK.

## What does openarchi already fetch?

PDOK for the BGT (OGC API features at `https://api.pdok.nl/lv/bgt/ogc/v1`), BAG v2 buildings (`https://api.pdok.nl/kadaster/bag/ogc/v2`, collection `pand`), BRK v1 parcels (`https://api.pdok.nl/kadaster/brk-kadastrale-kaart/ogc/v1`), AHN heights through WCS (`https://service.pdok.nl/rws/ahn/wcs/v1_0`, coverages `dsm_05m` and `dtm_05m`) and the Locatieserver for search. 3D BAG timed out from this machine and is not used.

Everything below is new.

## Where do business and facility points come from?

The first try was Overpass, the read-only query API for OpenStreetMap. One query for every `amenity`, `shop`, `office`, `tourism`, `leisure` and `public_transport=platform` object within 300 m of the Westerkerk:

```
POST https://overpass-api.de/api/interpreter
data=[out:json][timeout:25];(nwr["amenity"](around:300,52.3745,4.8840);nwr["shop"](around:300,52.3745,4.8840);nwr["office"](around:300,52.3745,4.8840);nwr["tourism"](around:300,52.3745,4.8840);nwr["leisure"](around:300,52.3745,4.8840);node["public_transport"="platform"](around:300,52.3745,4.8840););out center tags;
```

It returned 398 elements (132,577 bytes) in 19.6 s: 180 amenities, 153 shops, 51 tourism, 13 offices, 3 leisure. The top amenities were `restaurant` 38, `bench` 36, `cafe` 17, `bicycle_parking` 15, `fast_food` 8, `pub` 5. Each element looks like this, with `out center` giving ways and relations a single point:

```json
{"type": "node", "id": 34044750, "lat": 52.3738812, "lon": 4.8839268,
 "tags": {"brand": "Albert Heijn", "addr:street": "Westermarkt", "addr:housenumber": "21", "shop": "supermarket", ...}}
```

That is good enough to split into the two diagrams. Business is `shop=*`, `office=*`, `craft=*` and the food and drink amenities (`restaurant`, `cafe`, `bar`, `pub`, `fast_food`), plus `tourism=hotel`. Facility is education (`school`, `university`, `kindergarten`, `library`), culture (`theatre`, `cinema`, `arts_centre`, `tourism=museum`), civic (`townhall`, `police`, `post_office`, `place_of_worship`), health (`doctors`, `pharmacy`, `hospital`, `dentist`), leisure (`leisure=*`), transit stops and parking. Street furniture (`bench`, `waste_basket`, `vending_machine`) should be dropped, since it is a third of the amenity count and means nothing on a business map.

Overpass is the weak point. Over the session, `overpass-api.de` answered with `502` and "This Overpass API server is currently at capacity" twice, once for the transit query and once for the building query, and only a retry after a short wait succeeded. Of the alternative instances, `overpass.private.coffee` timed out after 60 s and `maps.mail.ru` returned `504`. The usage policy on the OSM wiki asks for fewer than 10,000 queries and less than 1 GB per day per user, a meaningful `User-Agent`, and backoff on `429` or `502`. Requests come from each user's own IP, so a desktop app spreads the load naturally, but openarchi must send one combined query per site (not one per diagram), cache the answer with the site, and retry with backoff.

The licence is ODbL 1.0. A diagram is a "produced work", so it needs the credit "© OpenStreetMap contributors" on the sheet; the share-alike part only applies if we redistribute the extracted database itself, which openarchi does not do.

The second option was Overture Maps places, which has no API but publishes GeoParquet files on S3 that can be read with HTTP range requests. Through DuckDB, a bbox query over `s3://overturemaps-us-west-2/release/2026-10-21.0/theme=places/type=place/*` for a 400 x 400 m box around the Westerkerk took 17.7 s and returned 674 places. Each row has a name, a `basic_category` and a `taxonomy.hierarchy`, for example `('Dr Spectacles', 'fashion_and_apparel_store', ['shopping', 'fashion_and_apparel_store', 'eyewear_store'], 0.89, 'POINT (4.8828386 52.3756645)')`. The top categories were `professional_service` 58, `fashion_and_apparel_store` 40, `restaurant` 37, `corporate_or_business_office` 35. 382 of the 674 came from BrightQuery, which is a business register, so many points are registered offices at home addresses rather than places you can walk into. 652 rows are CDLA-Permissive-2.0, 21 Apache-2.0, 1 CC0, all fine for a GPL app with attribution.

Overpass is the recommendation for round 2, because one query also returns transit stops and footpaths, and the OSM tagging is cleaner for facilities. Overture is a useful second source for business density in areas where OSM is thin, but it needs a Parquet reader in the Rust core and the release path changes monthly.

#! NOTE[poi, overture] : Is Overture worth a Parquet dependency in the core, or only as an optional "dense business" layer later?

## What about transit stops and walk times?

OSM again. Overpass for `public_transport=platform`, `highway=bus_stop`, `railway=tram_stop|station|halt` and route relations within 800 m of TU/e answered in 33.0 s (after one `502`) with 7 stop nodes, 8 platform ways and 58 route relations. The stops carry a name and the national stop code, for example `TU Eindhoven`, `ref:IFOPT` `NL:Q:64004011`. The route relations list the lines: bus `3`, `4`, `6`, `154`, `156`, `157`, `305`, `321`, `322`, `605`, `822`, `831`, `833`, night buses `N80`, `N831`, and the train services through Eindhoven Centraal. That covers "which lines stop within a 10 minute walk".

OVapi (`http://v0.ovapi.nl/stopareacode/`) answered in 0.20 s with 661,323 bytes for 4,577 stop areas, but 1,524 of them have the same placeholder position (lon 3.3135, lat 47.9748, in France), the HTTPS endpoint fails certificate checks (`curl` exit 60), and `http://v0.ovapi.nl/stopareacode/AmsWtm` returned `{"AmsWtm":{}}`. It is a real-time departures API, not a stop register, and is not usable here.

The national GTFS timetable at `http://gtfs.ovapi.nl/nl/gtfs-nl.zip` is 242,385,631 bytes and is regenerated daily (last modified 2026-10-23). It would give departures per hour per stop, but it is far too large to download per site. It could be cached once a month if a frequency diagram is ever wanted.

Walk times need a footpath network and a shortest path search. Public routing services (openrouteservice) need a free key and have daily quotas, so the plan is to compute it locally: fetch `highway=*` ways within about 1.2 km in the same Overpass query, build a graph in Rust, run Dijkstra from the site at 1.4 m/s (about 5 km/h), and draw 5, 10 and 15 minute isochrones. The same graph gives the 15 minute neighbourhood: count facilities by category inside each band.

That is how it was built (`crates/core/src/sources/osm/` and `crates/core/src/walk/`). The one query per site reaches 1,500 m around the middle of the 250 m tile the site centre falls in, so the cached answer serves any site centred in that tile; for a 300 m site at the Westerkerk the answer was 8.2 MB with 180 route relations, and the whole fetch, PDOK included, took 42 s. Two things came up on the way. Route relations are incomplete: at Westermarkt only the night buses list the stop, so a stop tagged `railway=tram_stop` stays a tram stop even when no tram route lists it. And Amsterdam maps pavements as separate footways, so they are walked but not drawn, or every street would show three lines.

#! NOTE[decision, poi, parking] : Car parks stay in Walkability, and bicycle parking joins them as its own group (2026-10-24)

#! NOTE[transit, gtfs] : Do we want departures per hour per stop? That is the only reason to touch the 242 MB GTFS file

## Is there an open traffic noise map?

Yes, and it is very good. RIVM publishes the Atlas Leefomgeving rasters through WMS (`https://data.rivm.nl/geo/alo/wms`, 254 layers) and WCS (`https://data.rivm.nl/geo/wcs`). The capabilities say `Fees NONE` and `AccessConstraints NONE`. The useful coverages are kept as an "actueel" alias that points at the latest year:

- `alo__rivm_Geluid_lden_wegverkeer_actueel` (road traffic, Lden)
- `alo__rivm_Geluid_lden_treinverkeer_actueel` (rail)
- `alo__rivm_Geluid_lden_allebronnen_actueel` (all sources combined)
- `alo__rivm_Geluid_lnight_wegverkeer_actueel` (road traffic at night)

The dated versions are also there, for example `alo__rivm_20250101_Geluid_lden_wegverkeer_2022`. The axes are called `X` and `Y` in RD (EPSG:28992), not `x` and `y` (the first try failed with `InvalidAxisLabel`), and `curl` needs `-g` because of the brackets:

```
https://data.rivm.nl/geo/wcs?service=WCS&version=2.0.1&request=GetCoverage&coverageId=alo__rivm_Geluid_lden_wegverkeer_actueel&subset=X(120500,120900)&subset=Y(487400,487800)&format=image/tiff
```

That returned a 5,013 byte float GeoTIFF in 0.21 s: 40 x 40 cells at 10 m, values 39 to 74 dB, mean 56.6 dB, nodata `-32768`. This is the same request shape as the AHN WCS openarchi already uses, so the noise diagram is mostly a new colour ramp.

The EU noise maps from the Environmental Noise Directive only cover major roads and agglomerations at a coarser level, and the RIVM map already includes them, so they were not tested.

The RIVM capabilities give no licence, and neither do the Atlas Leefomgeving terms of use (`Gebruiksvoorwaarden Atlasleefomgeving.pdf`, 2019), which only say that each map's own owner is responsible for it. data.overheid.nl does say it: dataset `5589` (road traffic noise, Lden) lists `rivm_20250101_Geluid_lden_wegverkeer_2022` as "Publiek domein", the Public Domain Mark 1.0, with RIVM as owner, and the 2024 NO2 concentration maps are listed the same way. openarchi credits "RIVM Atlas Leefomgeving" with that licence.

The "actueel" coverages give the dated coverage they point at as their first keyword in the capabilities. On 2026-10-24 road noise and all sources pointed at the 2022 maps (`alo__rivm_20250101_Geluid_lden_wegverkeer_2022`, `alo__rivm_20250801_Geluid_lden_allebronnen_2022`) and the air quality maps at 2024 (`alo__rivm_nsl_20260401_gm_NO22024` and the PM10 and PM2.5 versions). openarchi asks for the dated ids, because the legend has to name the year, and finds the newest one when a site is fetched (see "How does openarchi find the newest edition?" below). The road noise tiles are 16-bit integers, not floats, and the air quality grid starts 11.6 m east and 2.9 m south of round numbers (a 250 m tile from x 120 500 comes back from x 120 511.6), so every tile is placed by the georeferencing in its own GeoTIFF.

## Where does flood risk come from?

LIWO, the national flood information service of Rijkswaterstaat, runs a GeoServer at `https://basisinformatie-overstromingen.nl/geoserver/LIWO_Basis/ows` with WMS and WCS, `Fees none`, `AccessConstraints none`. The national maximum water depth comes in five layers, `MaximaleWaterdiepteNederland_Kaart1` to `Kaart5`. The Klimaateffectatlas carries the same data under clearer names (`maximale_waterdiepte_nederland_grote_kans`, `middelgrote_kans`, `kleine_kans`, `extreem_kleine_kans`), which suggests Kaart1 is the large probability scenario and Kaart5 the extremely small one.

A WMS `GetFeatureInfo` at the Westerkerk took 0.17 s and returned `{"GRAY_INDEX": -9999}` (dry) for Kaart1 and Kaart3, and `{"GRAY_INDEX": 0.1807}` (18 cm of water) for Kaart5. At TU/e all three were `-9999`. The WCS works with the same RD axes as RIVM:

```
https://basisinformatie-overstromingen.nl/geoserver/LIWO_Basis/ows?service=WCS&version=2.0.1&request=GetCoverage&coverageId=LIWO_Basis__MaximaleWaterdiepteNederland_Kaart5&subset=X(120500,120900)&subset=Y(487400,487800)&format=image/tiff
```

It returned 26,004 bytes in 0.34 s, 80 x 80 cells at 5 m, maximum 0.82 m in the window, nodata `-9999`. Rainfall flooding (the stress test layer `Bovenregionale_stresstest_wateroverlast_Waterdiepte`) returned no feature at the Westerkerk, so it does not cover every municipality.

The WMS layers have an empty abstract, so the capabilities do not say which probability each map stands for. LIWO's method report does: "Technische uitgangspunten en methode LIWO afgeleide kaarten" (Deltares, 11211536-000-ZWS-0003, 21 November 2025), table 2-1, numbers them 1 grote kans (more than 1 in 30 a year), 2 middelgrote kans (1 in 30 to 1 in 300), 3 kleine kans (1 in 300 to 1 in 3,000), 4 zeer kleine kans (1 in 3,000 to 1 in 30,000) and 5 extreem kleine kans (less than 1 in 30,000). Each map includes every scenario with a chance at least as large as its class, so Kaart5 is the fullest picture of where water can reach. The Flood depth diagram uses Kaart5 and says so in its legend. Over the 300 m around the Westerkerk it reaches 0.82 m and covers 35 % of the site.

#! NOTE[flood, licence] : The LIWO capabilities say "Fees none, AccessConstraints none" but name no licence; ask liwo@rws.nl which one applies

## How does openarchi find the newest edition?

Every dated source publishes a new year on its own schedule, and the services of other countries will too, so no year is pinned in code. When a site is fetched, each source asks its own service which editions it has, and the sheet, the ratios, the Analysis tab and `get_environment` all quote the year that was used. The scene keeps those years in `editions`, so a saved project goes on naming 2022 after RIVM publishes 2023; fetching the site again picks up the new year.

RIVM is asked through its WCS capabilities (`https://data.rivm.nl/geo/wcs?service=WCS&version=2.0.1&request=GetCapabilities&sections=Contents`). The document is 11.4 MB with about 15,000 coverages, 0.35 MB gzipped, and took about 1 s on 2026-10-24. Of the ids starting `alo__rivm_`, each map takes the newest year, and of that year the latest publication date, among ids shaped exactly like `alo__rivm_<yyyymmdd>_Geluid_lden_wegverkeer_<yyyy>` or `alo__rivm_nsl_<yyyymmdd>_gm_NO2<yyyy>`. The exact shape matters, because the list also has variants like `alo__rivm_nsl_20210101_gm_NO22019_Int_v2` and the undated "actueel" aliases. On 2026-10-24 this picked the same ids as before: road noise and all sources 2022, NO2, PM10 and PM2.5 2024. Rail noise already has 2023 and industry 2024, which openarchi does not draw yet.

CBS is asked with one square per year, `.../vierkant100/items?f=json&limit=1&jaarcode=2025`, going back from last year until one answers with `"numberReturned":1`. Each question takes about 50 ms; on 2026-10-24 2025 answered `"numberReturned":0` and 2024 answered with a square.

Both answers are kept in the tile cache for a day (`edition-rivm` and `edition-cbs`), so only the first fetch of a day asks, and both are asked at the same time, with a 5 s limit and no retries. When a service cannot be asked, the fetch uses the last answer it gave, or else the edition named in the source module, and adds a fetch warning that the year may not be the newest. Each edition's tiles are cached under their own id (`wcs-alo__rivm_nsl_20260401_gm_NO22024`, `cbs-vierkant100-2024`), so a new edition is downloaded fresh and never mixed with tiles of the old one.

For the uncached 1.32 km² site at Eindhoven Centraal (`fetch_site -- 5.4797 51.4433 648`), the PDOK, RIVM and LIWO tiles finished at 8.6 s before this change and at 9.5 s after it, of which 1.2 s was asking for editions at the start. The whole fetch took 50.9 s before and 51.2 s after, because the Overpass query (about 48 s) runs alongside. With every tile and the editions cached it took 1.8 s both before and after.

LIWO cannot be asked. Its WCS and WMS serve only the current release under undated names (`LIWO_Basis__MaximaleWaterdiepteNederland_Kaart5`), the DescribeCoverage has no metadata, and the WMS layers have no abstract or keywords, so openarchi still quotes the 2025 release that the method report describes. AHN is the same: PDOK's `dsm_05m` and `dtm_05m` carry no edition, and the slope legend names AHN4 by hand.

#! NOTE[flood, edition] : LIWO names no release in its services; ask liwo@rws.nl whether there is an endpoint that says which release Kaart5 shows

## Is there population and household data per block?

Yes, through PDOK in three resolutions, all CC BY 4.0 (credit "CBS").

The 100 m grid lives at `https://api.pdok.nl/cbs/vierkantstatistieken100m/ogc/v1/collections/vierkant100/items`. A bbox query without a year filter returned the oldest years first (2000 and 2001), because every year since 2000 is in the same collection. Adding `jaarcode` fixes that:

```
https://api.pdok.nl/cbs/vierkantstatistieken100m/ogc/v1/collections/vierkant100/items?f=json&bbox=4.8835,52.3740,4.8845,52.3750&jaarcode=2023
```

That took 0.96 s and returned three cells, for example `E1206N4875`: `aantal_inwoners` 140, `aantal_woningen` 85, `aantal_part_huishoudens` 85, `aantal_eenpersoonshuishoudens` 50, `gemiddelde_woz_waarde_woning` 813. `jaarcode=2024` is the newest (0.55 s, inwoners 135, WOZ `-99995` meaning not yet available) and `2025` returns nothing, so openarchi fetches the newest year it finds and the year before it, and takes each figure from the newest year that has it. The value `-99997` means suppressed for privacy, which happens in cells with few residents, so the diagram needs a "no data" class. Counts are rounded to 5. Fetching a whole 300 m site at once, some tile requests answered `500` even after one retry, and the same requests worked a minute later.

The 500 m grid (`.../vierkantstatistieken500m/ogc/v1/collections/vierkant500/items`) has the same fields and also 1971 (`aantal_inwoners_1971` 5550), but without a year filter it took 9.4 s.

Neighbourhood figures come from wijken en buurten, now up to 2025 (`https://api.pdok.nl/cbs/wijken-en-buurten-2025/ogc/v1/collections/buurten/items`, 0.09 s). The Westerkerk buurt is `Leliegracht e.o.`, `BU0363AC02`, with 1825 inwoners, 1240 huishoudens and an average household size of 1.5. The 2024 edition has much more: business counts by sector (`aantal_bedrijven_handel_en_horeca` 180), distances to facilities (`basisonderwijs_gemiddelde_afstand_in_km` 0.8), and car counts. Those distance fields are a ready-made cross-check for the 15 minute neighbourhood.

## Can zoning be fetched without a key?

Partly. Since the Omgevingswet started on 2024-01-01, new rules live in the omgevingsplan in the DSO (Digitaal Stelsel Omgevingswet). Both its API (`https://service.omgevingswet.overheid.nl/publiek/omgevingsdocumenten/api/presenteren/v8/regelingen`) and the ruimtelijkeplannen API v4 (`https://ruimte.omgevingswet.overheid.nl/ruimtelijke-plannen/api/opvragen/v4/plannen`) returned `401` ("Inloggegevens ontbreken") without a key. Both keys are free but need a registration on the developer portal, and the key would have to ship inside the app or be entered by each user.

The older zoning plans (bestemmingsplannen), which still apply in most places as part of the omgevingsplan, are open through PDOK at `https://service.pdok.nl/kadaster/plu/wms/v1_0` with `AccessConstraints` CC0. Layers include `enkelbestemming`, `dubbelbestemming`, `bouwvlak` and `bouwaanduiding`. `GetFeatureInfo` with `info_format=application/json` returns the actual polygon and its attributes. At the Westerkerk:

```json
{"bestemmingshoofdgroep": "gemengd", "naam": "Gemengd - 2", "planstatus": "vastgesteld",
 "dossierid": "NL.IMRO.0363.A1102BPSTD", "typeplan": "bestemmingsplan",
 "verwijzingnaartekst": "https://ruimtelijkeplannen.nl/documents/NL.IMRO.0363.A1102BPSTD-VG01/r_NL.IMRO.0363.A1102BPSTD-VG01_2.12.html"}
```

At TU/e the point fell on `Verkeer` (onherroepelijk, 2011). `GetFeatureInfo` only answers for one point, so drawing a whole zoning map needs either a grid of probe points or the WMS image itself as an underlay.

#! NOTE[zoning, key] : Register a free DSO API key, or ship zoning as the PDOK plan map only and label it "pre-2024 plans"?

## Where do climate normals come from?

Three services were tested, and each is best at something different.

Open-Meteo's historical API (`https://archive-api.open-meteo.com/v1/archive`) uses ERA5 reanalysis (about 25 km, from 1940) and ERA5-Land (0.1°, from 1950). Thirty years of daily mean, max and min temperature, precipitation, snowfall, max wind speed, dominant wind direction and shortwave radiation for the Westerkerk (1995-01-01 to 2024-12-31) came back in 1.57 s as 559,765 bytes of column arrays (`{"daily": {"time": [...], "temperature_2m_mean": [...], ...}}`). Averaged: 10.64 °C mean, 833 mm precipitation and 14.8 cm snowfall a year. The very next request, one year of hourly data for TU/e, failed in 0.16 s with `429` and "Minutely API request limit exceeded". The reason is in the pricing page: a request for more than 10 variables or more than 2 weeks counts as several calls, so 30 years of daily data is about 780 calls, over the free limit of 600 per minute (and 5,000 per hour, 10,000 per day). A minute later the hourly request worked in 0.67 s.

The bigger problem is the terms: the free API is "only for non-commercial purposes". openarchi itself is free, but an architecture office using it on a paid project is arguably commercial use. The data is CC BY 4.0 with credit "Weather data by Open-Meteo.com".

PVGIS from the EU Joint Research Centre has no key and no commercial restriction. Its typical meteorological year (`https://re.jrc.ec.europa.eu/api/v5_3/tmy?lat=51.4483&lon=5.4908&outputformat=json`, 1.62 s, 1,268,335 bytes) is 8,760 hourly rows built from the most typical month of 2005 to 2023, with exactly the fields a climate page needs:

```json
{"time(UTC)": "20210101:1200", "T2m": 4.13, "RH": 92.15, "G(h)": 208.0, "Gb(n)": 402.73, "Gd(h)": 102.0, "IR(h)": 257.6, "WS10m": 2.41, "WD10m": 239.0, "SP": 100620.0}
```

That is temperature, relative humidity, global horizontal (GHI), direct normal (DNI) and diffuse radiation, long-wave sky radiation (`IR(h)`), wind speed and wind direction, which covers the temperature chart, the outdoor comfort chart (UTCI needs all of air temperature, humidity, wind at 10 m, the three short-wave parts and `IR(h)`), the wind rose by season, humidity, solar radiation and the radiation rose in one call. `MRcalc` with `horirrad=1&mr_dni=1` gives monthly GHI and DNI per year (0.70 s; December 2023: `H(h)_m` 15.47 kWh/m², `Hb(n)_m` 18.53). PVGIS has no precipitation or snow.

KNMI's daily station data needs no key either:

```
POST https://www.daggegevens.knmi.nl/klimatologie/daggegevens
stns=240:370&vars=TG:TN:TX:RH:UG:FG:DDVEC:Q&start=20240101&end=20240107
```

It returned CSV in 0.19 s, for example `370,20240102,108,75,125,202,87,86,212,62` for Eindhoven on 2 January 2024 (mean 10.8 °C, precipitation 20.2 mm, humidity 87 %; values are in tenths). It has precipitation and measured wind, but only at about 35 stations (240 is Schiphol, 370 is Eindhoven), so openarchi would pick the nearest station and say so on the sheet.

The recommendation is PVGIS TMY for everything hourly, KNMI for precipitation, and Open-Meteo only for snowfall, cached per site and requested in one-year chunks to stay well under the per-minute limit.

#! NOTE[climate, licence] : Is Open-Meteo's non-commercial clause acceptable for an app architects use at work? If not, drop snowfall or take it from KNMI's few snow stations

## Is there open air quality data?

For a site analysis the right number is the annual mean, not today's value. RIVM publishes the national air quality maps (GCN) in the same WCS as noise: `alo__rivm_jaargemiddeld_NO2_actueel`, `alo__rivm_jaargemiddeld_PM10_actueel` and `alo__rivm_jaargemiddeld_PM25_actueel`, with dated versions up to `alo__rivm_nsl_20260401_gm_NO22024`. The NO2 request at the Westerkerk took 0.29 s and returned 16 x 16 cells at 25 m, 18.95 to 23.23 µg/m³.

Open-Meteo's air quality API (`https://air-quality-api.open-meteo.com/v1/air-quality?latitude=52.3745&longitude=4.8840&hourly=pm10,pm2_5,nitrogen_dioxide&past_days=7&domains=cams_europe`) answered in 0.18 s with hourly CAMS model values on a 0.1° grid (PM2.5 around 4 µg/m³, NO2 10 to 15 µg/m³). Luchtmeetnet (`https://api.luchtmeetnet.nl/open_api/concentrations?formula=NO2&latitude=52.3745&longitude=4.8840`, 0.19 s) gives the hourly forecast for a point. Both are live values, useful for a "today" chip but not for design advice. RIVM is the recommendation.

## Where do vegetation and heat come from?

Ready-made layers first. RIVM's green maps are in the same WCS: `alo__rivm_20240101_boomhoogte_2022` (tree height, 0.13 s, 80 x 80 cells at 5 m, maximum 24 m around the Westerkerk) and `alo__rivm_20240101_groenkaart_2022` (green classes 0 to 4). The WMS also has `RIVM_ndvi_sentinel_2022` and `rivm_20231221_bomenkaart_2022`. That is enough for a tree canopy diagram without any image processing.

For a true vegetation index, PDOK's colour infrared aerial photo (`https://service.pdok.nl/hwh/luchtfotocir/wms/v1_0`, layer `2025_ortho25IR`, 25 cm) returned an 800 x 800 image in 0.48 s. It came back as JPEG even when PNG was asked for. In infrared photos the red channel is near infrared and the green channel is red light, so an approximate NDVI is `(R - G) / (R + G)` per pixel. It is approximate because the image is colour-corrected for display, not calibrated reflectance.

Sentinel-2 is the calibrated option. The Earth Search STAC API (`https://earth-search.aws.element84.com/v1/search`) found three summer 2026 scenes over TU/e with under 10 % cloud in 0.83 s, and the band files are Cloud Optimised GeoTIFFs on public S3 (`.../S2C_31UFT_20260814_0_L2A/B04.tif`); a 16 KB range read took 0.90 s. NDVI from bands B08 and B04 at 10 m. The Copernicus Data Space and the Satellietdataportaal both need an account, so they were skipped.

For heat, the Klimaateffectatlas WMS (`https://cas.cloud.sogelink.com/public/data/org/gws/YWFMLMWERURF/kea_public/wms`, `Fees NONE`) has `hitteeiland`, the urban heat island effect in °C: 2.58 at the Westerkerk and 1.15 at TU/e (0.24 to 0.51 s). Its `gevoelstemperatuur_2022` (felt temperature) layer only returns a rendered colour (`RED_BAND`), not a value, so it can only be shown as an image. For measured ground surface temperature, Landsat Collection 2 Level-2 on Microsoft Planetary Computer works without an account: STAC search (`https://planetarycomputer.microsoft.com/api/stac/v1/search`, 0.59 s) found `LC09_L2SP_197024_20260802_02_T1` with 8.13 % cloud, and the asset URLs need a free anonymous token from `https://planetarycomputer.microsoft.com/api/sas/v1/token/landsat-c2-l2` that lasts about a day. Band `lwir11` is surface temperature at 30 m (100 m native).

#! NOTE[heat] : Landsat surface temperature needs scene picking (cloud, hottest recent day); start with the Klimaateffectatlas heat island layer and add Landsat later

## Is there open traffic intensity or congestion data?

Not for city streets. NDW publishes national feeds at `https://opendata.ndw.nu/`, for example `snelheden_en_intensiteiten_meetgegevens_en_configuratie_meetlocaties.xml.gz` (2,985,268 bytes, rewritten every minute) and `trafficspeed.xml.gz` (1,116,475 bytes). These are DATEX II snapshots from loop detectors, mostly on motorways and provincial roads. A single snapshot says nothing about typical congestion, and historic averages are behind the Dexter portal, which needs an account. The old NSL monitoring tool, which had traffic counts per city street, has been replaced, and its air quality output now lives in the RIVM maps above.

The practical answer is to draw traffic as a road hierarchy (OSM `highway` class, or BGT road function, which openarchi already has) and use the RIVM road noise map as the intensity proxy, since that map is computed from traffic volumes.

#! NOTE[traffic] : Is a road hierarchy plus noise good enough, or should we look at Dexter for historic counts on the bigger roads?

## How many storeys and households does each building have?

BAG does not record storeys. The verblijfsobject collection (`https://api.pdok.nl/kadaster/bag/ogc/v2/collections/verblijfsobject/items?bbox=...`) answered in 0.17 s with 29 units in a 70 x 110 m box around the Westerkerk, each with `gebruiksdoel` (use), `oppervlakte` (floor area in m²), `status` and `pand.href`, a link to the building. 8 of the 29 had status `Verblijfsobject ingetrokken` (withdrawn) and must be filtered out. Counting units with `woonfunctie` per `pand.href` gives dwellings per building, which is the best open proxy for households; summing `oppervlakte` per building gives the gross floor area that the floor area ratio needs. Licence: Public Domain Mark 1.0.

OSM storeys are too sparse to rely on: of 1,197 buildings within 300 m of the two sites (24.2 s), only 56 had `building:levels`. Storeys are better estimated locally as AHN roof height minus ground height, divided by a storey height (3 m for housing, 3.5 to 4 m for offices), and 3D BAG would give the same with less noise if it ever answers.

#! NOTE[storeys] : Retry 3D BAG from another network; it would replace the local height estimate

## Where do the country and municipality outlines come from?

PDOK's administrative boundaries, CC BY 4.0 with credit "Kadaster". `https://api.pdok.nl/kadaster/bestuurlijkegebieden/ogc/v1/collections/gemeentegebied/items?bbox=5.4900,51.4478,5.4916,51.4488` took 0.13 s and returned Eindhoven (`GM0772`, province `Noord-Brabant`) as a 135,201 byte polygon. `landgebied` (the Netherlands) is 1,439,726 bytes in 0.39 s, which is worth bundling with the app at reduced detail instead of fetching each time. `provinciegebied` is also there.

## How cheaply can the neighbourhood around a site get building heights?

The neighbourhood drawn around a site needs a height per building for the axonometric, but not a roof shape, so 0.5 m AHN cells would be wasted on it. The PDOK AHN WCS takes the WCS 2.0 scaling parameter. Asking for one 250 m tile of `dsm_05m` at `x(120500,120750)`, `y(487500,487750)` returned 705,149 bytes at full resolution in 0.24 s, and with `scaleFactor=0.25` a 125 x 125 cell GeoTIFF of 2 m cells, 50,519 bytes, in 0.13 s. `dtm_05m` scaled the same way was 30,073 bytes. openarchi fetches the neighbourhood's heights this way and caches them as `ahn-dsm_05m-coarse` and `ahn-dtm_05m-coarse`, apart from the full-resolution tiles of the site.

Sections reuse those 2 m terrain tiles; no new request is made. The scene keeps them over the whole neighbourhood square in larger cells (the median of each block of measured cells, to the decimetre): 4 m cells and 226 by 226 heights for a 200 m site, and never more than 300 cells across. Measured on Westermarkt and on the Valkhof in Nijmegen (200 m each), that added 36 kB and 39 kB to the packed scene, about 3 %. When the neighbourhood could not be fetched, the site's own 0.5 m terrain is kept the same way. `dtm_05m` has no value under buildings and water; the section profile bridges the gaps under buildings in a straight line, and water is drawn as described below.

The neighbourhood's buildings, roads, water and green come from the same BAG `pand` and BGT `wegdeel`, `waterdeel` and `begroeidterreindeel` collections as the site, in the same 250 m tiles, so the tiles along the site's edge are fetched once for both. For the 648 m circle at the TU/e campus in Eindhoven (1.32 km²) on an empty cache, that adds 468 tile requests to the 799 the site already makes. Over three runs the tiles finished after 10.3 to 11.7 s without the neighbourhood and 15.5 to 18.0 s with it, while the one Overpass request, which runs alongside them, took 29.7 to 73.6 s and set the total either way.

## How deep is the water, and at what level does it stand?

A section that crosses a canal or a river needs two numbers the terrain does not have: where the water surface is, and where the bed is. AHN gives neither. Its terrain model `dtm_05m` has no value over water, and the surface model is empty too: over the Waal at the Valkhof (X 188000 to 188100, Y 429200 to 429250) all 20,000 cells of `dsm_05m` were nodata on 2026-10-25. Over a stretch of Prinsengracht most cells had values, but those are boats and quay edges, not a water level.

The bed comes from Rijkswaterstaat's "Bathymetrie Nederland - binnenwateren 1 mtr", a WCS at `https://geo.rijkswaterstaat.nl/services/ogc/gdr/bodemhoogte_1mtr_historie/ows` with axes `X Y` in RD New, 1 m cells in metres above NAP, and nodata `3.4e38`. Every month is its own coverage, `bodemhoogte_1mtr_historie__bodemhoogte_1mtr_YYYYMM`; the capabilities listed 202412 to 202602 on 2026-10-25, and data.overheid.nl lists the dataset under CC0 1.0. A 250 m tile is about 262 kB and took about 1.9 s. It covers the waters Rijkswaterstaat manages and nothing else: the Valkhof tile over the Waal (X 188000 to 188250, Y 429000 to 429250) had 24,998 of 62,500 cells, from 0.79 to 5.72 m NAP; a tile over the IJ (X 121250 to 121500, Y 488500 to 488750) had 1,694 cells from −8.28 to −3.35 m; the Westermarkt tile over the Amsterdam canals (X 120500 to 120750, Y 487500 to 487750) had none. Asking for 4 m cells with `scaleFactor=0.25` returns cells of 3.99997 by 3.99996 m, which the GeoTIFF reader refuses, so openarchi fetches 1 m tiles and makes the cells larger itself, the same way as the kept terrain. There is also a 20 m survey of the coast, `bodemhoogte_20mtr`, which no site needs yet.

The level comes from the fairway network, VNDS, as an OGC API at `https://geo.rijkswaterstaat.nl/services/ogc/gdr/fis_vnds/ogc/features/v1`, also CC0 1.0. Its collection `streefpeil` holds target levels as lines along fairways, with the level in `value` in metres above NAP; it answers a `bbox` in RD New with `bbox-crs` and `crs` set to EPSG 28992. Around Westermarkt it returned 12 lines, all "Amstellands boezempeil" at −0.4; around the Valkhof only "Stuwpeil Grave, Maas-Waalkanaal" at 7.95, which runs through no water near the site. A free-flowing river like the Waal has no target level at all, because its level follows the discharge. The neighbouring collection `waterstandstreefpeil` has deviations but no level, and `waterhoogte_nap` has points such as Nijmegen haven with a high of 1.6, a low of −1.4 and an average of 7.7, but its `referencelevel` is `UNSPECIFIED`, so it is not used. The water boards' `peilgebied` polygons on PDOK (`api.pdok.nl/hwh/waterschappen-waterbeheergebieden-imwa/ogc/v1`) name the level areas, AGV `93-1` for the Amsterdam canals for example, but carry no level.

So a fetch asks both services for each 250 m tile that BGT water reaches, after the scene is built, and caches each answer per tile like every other source. The bed is kept like the terrain, in 4 m cells to the decimetre, which for the Valkhof is 227 by 153 cells and about 6 kB compressed; the levels are kept as their lines. A section draws each water it crosses at the target level when a fairway line with one runs through it, and otherwise at the lower of its two banks, read from the terrain bridged across the water, since AHN measures the ground down to the water's edge. Where the bed was surveyed the water goes down to it; where it was not, the water fades out downwards. The legend says which: "Water at target level −0.40 m NAP", "Water at its bank, no published level", "Water depth not surveyed". With both services failing a fetch still succeeds, with a warning, and sections draw ground bridged across the water as before.

#! NOTE[sections, water] : Should the Waal be drawn at a measured level instead of its bank? Rijkswaterstaat's water level API has live and average levels per station, but none of them is a fixed level to draw

#! NOTE[sections, water] : City canals have no bed survey; do Waternet or the water boards publish canal depths openly?

## What can be computed locally instead of fetched?

AHN `dsm_05m` and `dtm_05m` plus BAG footprints already cover a lot, and computing them avoids new services entirely:

- Slope and aspect from the DTM, with a 3 x 3 window per cell.
- Sun path from date, time and latitude (a solar position formula, no data).
- Shadows for a date and time by casting rays across the DSM toward the sun.
- Sunlight hours by repeating the shadow test every 15 minutes over a day or a season.
- Sky view factor by sampling the horizon angle in 16 to 36 directions per cell over the DSM.
- View (what can be seen from a point) as a viewshed over the DSM.
- Storeys from DSM minus DTM per footprint, divided by a storey height.
- Floor area ratio and building coverage from BAG footprints, storeys and the site area.
- Walk isochrones and the 15 minute neighbourhood from the OSM footpath graph described above.
- Solar radiation on surfaces by combining the PVGIS hourly GHI, DNI and diffuse values with the shadow and sky view rasters.

## In which order should round 2 add them?

1. Business, facility and transit points from one cached Overpass query per site, with backoff and the OSM credit on the sheet. This unlocks three diagrams and the walk graph at once.
2. Traffic noise and annual air quality from the RIVM WCS, which reuses the AHN WCS code path.
3. Population, dwellings and households from the CBS 100 m grid (with `jaarcode`) and wijken en buurten 2025, plus BAG verblijfsobject counts per building.
4. Locator map from PDOK administrative boundaries, with the national outline bundled.
5. Climate page from PVGIS TMY and KNMI daily data, with Open-Meteo snowfall behind the licence decision above.
6. Flood depth from the LIWO WCS.
7. Walk isochrones and the 15 minute neighbourhood, computed locally from the footpath graph fetched in step 1.
8. Tree canopy and heat island from the RIVM green WCS and the Klimaateffectatlas WMS.
9. Zoning from the PDOK plan map, and the DSO API once a key decision is made.
10. Calibrated NDVI from Sentinel-2 and ground surface temperature from Landsat, which need scene selection and are the most work for the least gain.
