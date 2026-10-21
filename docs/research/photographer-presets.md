# Photographer "inspired by" presets: documented darkroom practice

Scope: the five "inspired by" presets (Don McCullin, Ansel Adams, Bill Brandt, Josef Koudelka, Edward Weston).
The brief says presets must "represent documented aesthetic interpretations, not unsupported claims about the precise
equipment or chemicals used", and "where historical information is available, use it to inform the recipes".

Retrieval date for all sources: **2026-10-21**.

Evidence tags (same idea as `tmax-d96-fx55.md`):

- **[P]**: primary or authoritative: the photographer's own words (interview, own statement or book), or text from a museum, the estate or gallery, the agency, or the photographer's printer or assistant.
- **[S]**: secondary: blog, forum, dealer or encyclopedia. Use it as a lead, not as a fact the app states.
- **not documented**: nothing usable was found. The recipe value is therefore pure interpretation.

Every recipe value is an **interpretation** of the look. No preset claims to reproduce the photographer's actual film,
chemistry or paper. The app's components cover film, film developer, push/pull, agitation, format, enlargement, paper
grade, print exposure and scene contrast. It has **no paper-developer, toner, filter or dodge/burn control**. Where
the record says those mattered (amidol, selenium, yellow or red filters, heavy burning), the closest proxy is noted.

App vocabulary used below: film ids as in the app; push in whole stops relative to box speed; format = film width
in mm (36 = 35mm, 56 = 6x6/645 roll film, 70 = 6x7, 120 = 4x5, 203 = 8x10); crop = enlargement crop factor
(1.0 = full frame); paper grade 0-5 (2 = normal); print exposure in stops (+ = darker); scene contrast -100..+100.

---

## 1. Don McCullin

### Documented facts

| Topic | Fact | Tag | Source |
|-------|------|-----|--------|
| Film | "I always used to go to Vietnam with thirty rolls of Tri-X film – nothing more, nothing less." | [P] | Aaron Schuman, "Don McCullin: The Extended Interview", https://www.aaronschuman.com/mccullinarticle.html |
| Camera / format | "I carried a Nikon F under my overcoat"; "there was a bullet-hole in my Nikon" (35mm). First camera was a Rolleicord bought on RAF national service | [P] | Schuman interview (above); Tate Liverpool, *Don McCullin* exhibition guide, https://www.tate.org.uk/whats-on/tate-liverpool/don-mccullin/exhibition-guide |
| Lens | "The longest lens I have 105mm" | [P] | "Interview with Don McCullin: A Confession of War" (2006), American Suburb X, https://americansuburbx.com/2013/05/interview-don-mccullin-a-confession-of-war-2006.html |
| Exposure habit | Meters carefully, even under fire: "I've stood up in battles and put up the exposure meter first", and would not risk "an underexposed negative" | [P] | Apollo, "'I've earned my reputation out of other people's downfall'", https://apollo-magazine.com/don-mccullin-photographer-interview/ |
| Filter | Uses a **yellow filter** on the camera, plus "careful choice of paper in the darkroom", to heighten contrast in the landscapes | [P] (journalist's description) | Apollo (above) |
| Print darkness | "Darkness, to me, is energy." He describes injecting more darkness into his prints in the darkroom, and calls it strength | [P] | American Suburb X 2006 interview (above) |
| Dodging / burning | "By dodging and burning, I engage in the landscape work"; holds back the eyes on a portrait where "it is complete darkness on the negative" | [P] | Schuman interview; ASX 2006 interview |
| Printing labour | "It takes 5 or 6 hours for me to make a print"; "I wash my prints for two hours"; tears up most attempts | [P] | ASX 2006 interview |
| Own printing | Every print in the Tate exhibition was printed by McCullin himself, in his home darkroom (a barn in Somerset) | [P] | Tate exhibition guide; Apollo |
| Landscape look | Winter Somerset landscapes, "Wagnerian" and "metallic skies", leafless trees; "I look for drama"; Tate: "dark, metallic skies resemble battlefields" | [P] | Schuman interview; Tate exhibition guide |
| Late career | Said in 2025 that his "printing days are over" | [P] | The Art Newspaper, 4 Feb 2025, https://www.theartnewspaper.com/2025/02/04/don-mccullin-on-swapping-war-zones-for-museums |
| Developer, paper brand/grade, toning | **not documented** in the sources found | | |

### Proposed preset: "McCullin — Darkness Is Energy"

| Parameter | Value |
|-----------|-------|
| Film | `tri-x` |
| Developer | `hc110-b` |
| Push/pull | 0 |
| Agitation | every 30 s |
| Format | 36 (35mm) |
| Enlargement crop | 1.2 |
| Paper grade | 3 |
| Print exposure | +1 stop |
| Scene contrast | +20 |

**Look:** dense, brooding 35mm Tri-X prints with heavy, energetic shadows and metallic, threatening skies.

**Basis:** Tri-X, the Nikon F (35mm) and his careful metering are documented in his own words (Schuman interview,
Apollo). So are deliberately darkened prints ("Darkness, to me, is energy", ASX 2006) and a yellow filter with
paper chosen for contrast (Apollo). The developer is not documented: HC-110 B is our choice, a vigorous, slightly
grainy rendering for a 1960s-70s photojournalist. Grade 3, +1 stop print exposure and +20 scene contrast are our
proxies for his darker printing and the yellow-filter sky. The app cannot model dodging or burning.

---

## 2. Ansel Adams

### Documented facts

| Topic | Fact | Tag | Source |
|-------|------|-----|--------|
| Method | Co-devised the **Zone System** (with Fred Archer, c. 1939-40): exposure placement plus development control (N+/N-) to set negative contrast | [S] (well established; primary is Adams, *The Negative*) | Wikipedia, "Fred R. Archer", https://en.wikipedia.org/wiki/Fred_R._Archer |
| Developer | HC-110 "is recommended by Ansel Adams in his book, The Negative" | [S] citing primary | Covington, "Kodak HC-110 Developer – Unofficial Resource Page", https://www.covingtoninnovations.com/hc110/index.html |
| Compensating development | Reported to use HC-110 dilution G (1+119) as a compensating developer, about 18 min at 20 °C, sparse agitation (15 s every 3 min); the film is "presumably" Tri-X | [S] | Covington HC-110 page (above) |
| *Moonrise, Hernandez* (1941): format | 8x10 camera | [P] (estate gallery) | The Ansel Adams Gallery, "The Making of 'Moonrise, Hernandez': A Legend in Light", https://articles.anseladams.com/a-legend-in-light/ |
| *Moonrise*: exposure | "I placed this value [the moon, 250 cd/ft²] on Zone VII" (Adams, *Examples: The Making of 40 Photographs*, 1983, pp. 41-43) | [P] quoted | Kean University Galleries, *Ansel Adams: Early Works* exhibition text, https://galleries.kean.edu/sites/default/files/2021-11/Ansel%20Adams%20Early%20Works%20Exhibition.pdf ; Wikipedia, https://en.wikipedia.org/wiki/Moonrise,_Hernandez,_New_Mexico |
| *Moonrise*: development | Water-bath (compensating) development, chosen because he feared the brightness range was too long. Agfa **Isopan** film and diluted **D-23** are reported, but only in secondary accounts | water bath [P] via *Examples*, reported; film/developer [S] | Ansel Adams Gallery articles (above); Photo.net forum thread, https://www.photo.net/forums/topic/468415-%E2%80%9Cmoonrise-hernandez-new-mexico%E2%80%9D/ |
| Printing evolution | Style moved from soft, warm, matte "Parmelian prints" (1920s) to, after the war, "a cooler, higher-contrast printmaking" style. Later work is described as "the heroic, high-contrast prints on high-gloss paper stock that Adams manufactured to order in the 1970s-80s" | [P] (university gallery) | Kean University exhibition text (above) |
| Print key | From the 1950s *Monolith* was printed with "the sky and most of the cliff side … jet black". Later *Moonrise* prints darken the sky "to almost black" | [P] | Kean exhibition text; Ansel Adams Gallery article |
| Paper / developer / toning | Preferred unferrotyped fibre-base papers and never resin-coated. Dektol was his standard print developer, Selectol-Soft was blended in for contrast control, and prints were toned in diluted Kodak Rapid Selenium toner (deeper blacks, permanence) | [P] (Alan Ross, Adams's assistant, who still prints the Yosemite Special Edition prints "basically the same" way) | Alan Ross, "Making the Ansel Adams Yosemite Special Edition Prints", https://www.alanrossphotography.com/ansel-adams/yosemite-special-edition-photographs/making-the-ansel-adams-yosemite-special-edition-prints/ |
| Enlarger / dodging | Diffusion light source, homemade dodging and burning tools, metronome-timed exposures | [P] (Ross) | Ross (above) |
| Specific paper grade (e.g. Oriental Seagull grade 4) | anecdotal only | [S] | Photrio forum, https://www.photrio.com/forum/threads/what-b-w-paper-did-ansel-adams-use.167318/ |

### Proposed preset: "Adams — Zone System Landscape"

| Parameter | Value |
|-----------|-------|
| Film | `fp4` |
| Developer | `hc110-h` |
| Push/pull | 0 |
| Agitation | every 60 s |
| Format | 203 (8x10) |
| Enlargement crop | 1.0 |
| Paper grade | 3 |
| Print exposure | +0.5 stop (round to the nearest supported step) |
| Scene contrast | +15 |

**Look:** full-scale large-format tonality with open shadows, held highlights and dramatically deepened skies, as in his post-war "heroic" prints.

**Basis:** Documented: the Zone System, the 8x10 format of *Moonrise*, compensating (water-bath) development, his
recommendation of HC-110 in *The Negative*, and the move to cooler, higher-contrast glossy prints with near-black
skies (Kean University exhibition text; Ansel Adams Gallery; Alan Ross). Substitutions: Isopan and D-23 are not in
the app. FP4 stands in as a classic medium-speed cubic-grain film. HC-110 at dilution H with sparse agitation is our
nearest compensating option (the documented dilution G and the water bath are not available). Grade 3, +0.5 print
exposure and +15 scene contrast approximate the later printing key and filtered skies. Selenium toning cannot be modelled.

---

## 3. Bill Brandt

### Documented facts

| Topic | Fact | Tag | Source |
|-------|------|-----|--------|
| Contrast shift | "Before 1951, I liked my prints dark and muddy. Now I prefer the very contrasting black-and-white effect." | [P] (Brandt's own statement) | "A statement on photography by Bill Brandt", Bill Brandt Archive, https://www.billbrandt.com/bill-brandt-a-statement |
| Darkroom role | "I find the darkroom work most important, as I can finish the composition of a picture only under the enlarger." He rejects rules against cropping | [P] | Brandt statement (above) |
| Own printing | "I consider it essential that the photographer should do his own printing and enlarging." | [P] quoted | The Art Story, https://www.theartstory.org/artist/brandt-bill/ |
| Reprinting | The prints for *Shadow of Light* (1966) "showed such a high degree of contrast" that early documentary images became near-abstract patterns. In the 1960s he reprinted earlier work with much harsher contrast, "sacrificing the mid-tones" (David Campany) | [P] (ICP) / [S] (blog quoting Campany) | International Center of Photography, https://www.icp.org/browse/archive/constituents/bill-brandt ; Art Blart on Fundación MAPFRE show, https://artblart.com/2021/08/22/exhibition-bill-brandt-at-the-fundacion-mapfre-madrid/ |
| Cameras | "Mostly used a Rolleiflex camera on a tripod", then "acquired a Hasselblad with a superwide-angle lens" (both 6x6 roll film) | [S] (review of MoMA 2013 show) | Time Out New York, https://www.timeout.com/newyork/art/bill-brandt-shadow-and-light |
| Nudes camera | "A 70-year-old wooden Kodak" wide-angle with no shutter, focused at infinity, used for *Perspective of Nudes* (1961); discarded for the book's last chapter | [P] | Brandt statement (above); V&A, https://collections.vam.ac.uk/item/O1061571/nude-photograph-brandt-bill/ |
| Paper | "grade 4 extra hard paper" from the mid-1950s; mostly 8x10 in single-weight glossy | [S] (unsourced blog) | Photogpedia, https://photogpedia.com/bill-brandt/ |
| Film / developer | **not documented** (one blog says "400 ASA", unsourced) | [S] | Photogpedia (above) |

### Proposed preset: "Brandt — Shadow of Light"

| Parameter | Value |
|-----------|-------|
| Film | `hp5` |
| Developer | `d76-stock` |
| Push/pull | +1 |
| Agitation | every 30 s |
| Format | 56 (6x6 roll film) |
| Enlargement crop | 1.3 |
| Paper grade | 5 |
| Print exposure | +0.5 stop |
| Scene contrast | +40 |

**Look:** graphic, near-abstract black-and-white with crushed mid-tones, inky shadows and chalky highlights, the post-1951 "very contrasting" Brandt.

**Basis:** Documented: his own statement that from 1951 he preferred "the very contrasting black-and-white effect" and
finished compositions under the enlarger (billbrandt.com), the high-contrast *Shadow of Light* reprints (ICP), and his
use of 6x6 Rolleiflex/Hasselblad cameras (Time Out on the MoMA show). Interpretation: film and developer are not
documented. HP5 (a British 400 film) in D-76 pushed +1 is our choice for a contrasty, slightly gritty negative. Grade
5 goes one step past the reported (secondary) grade 4, to stand in for his practice of "sacrificing the mid-tones".
Crop 1.3 reflects his documented cropping.

---

## 4. Josef Koudelka

### Documented facts

| Topic | Fact | Tag | Source |
|-------|------|-----|--------|
| Film (Czechoslovak era, to 1968) | For the 1968 invasion he used "a type of film that was for the cinema, which had no numbers on it" | [P] (Koudelka, Aperture interview) | Aperture, "Invasion 68: Prague" (2008), https://archive.aperture.org/article/2008/3/3/invasion-68-prague |
| Film handling | Rolls of cinema film bought cheaply from a friend, cut into strips and loaded by hand | [P] (journalist profile) | Sean O'Hagan (Guardian), reproduced at https://www.umilta.net/prague.html |
| Gypsies: film/dev | Shot on "East German 400 ASA movie film", at most 1/30 s, "pushed it as far as possible in a hot developer" | [S] | Shooter Files, "Master Profiles: Josef Koudelka", https://shooterfiles.com/2015/11/master-profiles-josef-koudelka/ |
| Negative character | His printer Voja Mitrović says the Czechoslovak-era negatives were often on cinema stock, "very high in contrast", with extreme tonal ranges needing heavy burning and dodging. His prints are "10 to 100 times more difficult to make" than Cartier-Bresson's | [S] reporting [P] (printer interview, *The Online Photographer*) | T.O.P., "Voja Mitrovic, Printer to the Greats (Part I/II)", https://www.patreon.com/theonlinephotographer/posts/voja-mitrovic-to-158110873 , https://www.patreon.com/theonlinephotographer/posts/voja-mitrovic-to-158112402 |
| Print intent | Wanted "relatively high contrast but with detail in both the highlights and shadows" | [S] reporting [P] (Mitrović) | T.O.P. (above) |
| Printer | Mitrović printed *Exiles*, *Black Triangle*, *Koudelka* and *Chaos*; also printed for Cartier-Bresson at Picto | [S] | Leica Stories, "Paris Printers", https://leicastories.com/paris-printers/ ; T.O.P. |
| Camera / lens (1960s) | Exakta (Varex) with a 25 mm f/4 Flektogon, 35mm. Koudelka: the 25 mm lens (1963) "changed my vision", giving "the full depth of field which I had always wanted" in poor light | [P] (Aperture) / [S] (model) | Aperture (above); Casual Photophile, https://casualphotophile.com/2019/05/13/five-famous-photographers-and-the-cameras-they-used-part-two/ |
| Later cameras | Leicas with 35/50 mm after exile; 6x17 panoramic cameras (Fuji 617 and others) for *Black Triangle* / *Industries* | [S] | Casual Photophile; Mechanical Landscapes, https://blog.mechanicallandscapes.com/2021/05/18/597-the-art-of-the-panorama-part-1-joseph-koudelkas-industries/ |
| Processing | Invasion negatives were developed months later. In 1970 in London he arrived at David Hurn's darkroom "with 800 rolls" | [P] | Aperture; O'Hagan profile |
| Specific developer, paper | **not documented** | | |

### Proposed preset: "Koudelka — Gypsies / Exiles"

| Parameter | Value |
|-----------|-------|
| Film | `double-x` |
| Developer | `hc110-b` |
| Push/pull | +2 |
| Agitation | every 30 s |
| Format | 36 (35mm) |
| Enlargement crop | 1.0 |
| Paper grade | 3 |
| Print exposure | +0.5 stop |
| Scene contrast | +25 |

**Look:** stark, grainy 35mm wide-angle frames with velvety blacks and hard, glowing highlights, contrasty but holding detail at both ends.

**Basis:** Documented: cinema film in the Czechoslovak years (Koudelka, Aperture 2008), 35mm with a 25 mm wide-angle,
and very high-contrast negatives printed to "relatively high contrast" with detail at both ends (printer Voja Mitrović,
via *The Online Photographer*). Substitution: the East German stock (unnamed, likely ORWO) is not in the app. Eastman
`double-x` is chosen because it is itself a motion-picture negative film. Push +2 (EI 1000 from 250) with HC-110 B and
frequent agitation interprets the secondary report of pushing "as far as possible in a hot developer". Grade 3 rather
than 4 stands in for Mitrović's heavy burning and dodging, which held detail; the app cannot model that.

---

## 5. Edward Weston

### Documented facts

| Topic | Fact | Tag | Source |
|-------|------|-----|--------|
| Cameras | "He had, and has, two cameras: an 8 x 10 view camera and a Graflex for portraits" | [P] (MoMA catalogue) | Nancy Newhall, *The Photographs of Edward Weston*, Museum of Modern Art, 1946, https://www.moma.org/documents/moma_catalogue_2374_300062041.pdf |
| Materials | "He uses one film, one paper" (not named) | [P] | Newhall 1946 |
| Film development | "He tray-develops his negatives in pyro-soda, by inspection, bringing each one to the exact degree of delicacy or density he wants" | [P] | Newhall 1946; also Kim Weston (grandson), https://kimweston.com/news/weston-photography-focusing-in-edwards-fingernails ("used Pyro to develop his film") |
| Printing | "Working under an overhead bulb with a printing frame, he makes only contact prints, dodging in areas beyond the scale of paper" | [P] | Newhall 1946 |
| Paper developer | "Developed in amidol … more than half of the first prints from his negatives are exhibition quality" | [P] | Newhall 1946; Kim Weston ("Amidol to develop his paper") |
| Paper | Azo (silver chloride contact paper) | [S] (forum citing the *Daybooks*) | Photrio forum, https://www.photrio.com/forum/threads/advantages-and-history-azo-silver-chloride-paper.54324/ |
| Surface | Moved from matte palladium to **glossy** prints: "the shells and rocks demanded a brilliance and clarity beyond the bronze tones and matte surface of the palladiotype" | [P] | Newhall 1946 |
| Retouching | From 1934 "all portraits are unretouched contact prints" | [P] | Newhall 1946 (chronology) |
| Film stock | **not documented** (forum guesses only) | [S] | Large Format Photography Forum, https://www.largeformatphotography.info/forum/archive/index.php/t-14058.html |

### Proposed preset: "Weston — 8x10 Contact"

| Parameter | Value |
|-----------|-------|
| Film | `fp4` |
| Developer | `rodinal-1+50` |
| Push/pull | 0 |
| Agitation | every 60 s |
| Format | 203 (8x10) |
| Enlargement crop | 1.0 |
| Paper grade | 2 |
| Print exposure | 0 |
| Scene contrast | 0 |

**Look:** grainless, razor-sharp 8x10 contact-print clarity with luminous, long-scale highlights and glossy brilliance in the forms.

**Basis:** Documented: an 8x10 view camera, contact prints only (crop 1.0), pyro-soda development by inspection, amidol
print development and glossy prints from the 1930s (Newhall, MoMA 1946; Kim Weston). Substitution: the app has no pyro.
Rodinal 1+50 is the closest available. It is a high-acutance developer that keeps edges sharp and, at this dilution
with intermittent agitation, compensates highlights somewhat; it does not reproduce pyro's stain. The film is not
documented: FP4 is our choice as a traditional medium-speed sheet-film emulsion. Grade 2 with neutral print exposure
reflects his "exact degree of delicacy or density". Amidol (a paper developer) and Azo cannot be modelled.

---

## Gaps and follow-ups

- Primary books were not read directly: Adams, *The Negative*, *The Print* and *Examples*; Weston, *Daybooks*;
  Brandt, *Camera in London* and *Shadow of Light*. Reading them would confirm the [S] items: Isopan and D-23 for
  *Moonrise*, Brandt's paper grade, Weston's Azo.
- The Koudelka film and printing details rely on The Online Photographer's Mitrović interview (paywalled; read via
  search summaries) and one secondary profile for "pushed in a hot developer".
- McCullin's developer, paper and toning are undocumented in the sources found.
