# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "marimo",
#     "polars",
#     "altair",
# ]
#
# [tool.marimo.display]
# theme = "dark"
# ///

import marimo

__generated_with = "0.23.2"
app = marimo.App(width="full")


@app.cell
def _():
    import marimo as mo
    import polars as pl
    import altair as alt
    import sqlite3
    from pathlib import Path

    DB_PATH = Path.home() / ".cache/lapresse-tui/lapresse-tui.db"
    conn = sqlite3.connect(f"file:{DB_PATH}?mode=ro", uri=True)
    mo.stop(not DB_PATH.exists(), f"Database not found at {DB_PATH}")
    return alt, conn, mo, pl


@app.cell
def _(conn, mo, pl):
    overview_total = pl.read_database("SELECT COUNT(*) as n FROM articles", conn).item()
    overview_range = pl.read_database(
        "SELECT MIN(published_at) as min_d, MAX(published_at) as max_d FROM articles",
        conn,
    )
    overview_min = overview_range.item(0, "min_d")[:10]
    overview_max = overview_range.item(0, "max_d")[:10]
    overview_sections = pl.read_database(
        "SELECT COUNT(DISTINCT section) as n FROM articles WHERE section IS NOT NULL",
        conn,
    ).item()
    overview_authors = pl.read_database(
        "SELECT COUNT(DISTINCT author) as n FROM articles WHERE author IS NOT NULL",
        conn,
    ).item()
    overview_images = pl.read_database(
        "SELECT COUNT(*) as n FROM images WHERE data IS NOT NULL",
        conn,
    ).item()
    mo.md(
        f"""
        # Vingt ans de *La Presse*

        **{overview_total:,}** articles. De **{overview_min}** à **{overview_max}**.
        {overview_sections:,} rubriques, {overview_authors:,} auteurs, {overview_images:,} images.

        ---
        """
    )
    return


@app.cell
def _(mo):
    mo.md("""
    ## La machine médiatique

    Chaque jour, des dizaines d'articles sortent des salles de rédaction.
    Mais à quel rythme exactement ? Et ce rythme a-t-il changé en 20 ans ?
    """)
    return


@app.cell
def _(alt, conn, pl):
    yearly_volume = pl.read_database(
        """
        SELECT substr(published_at,1,4) as year, COUNT(*) as articles
        FROM articles INDEXED BY idx_articles_published_at
        GROUP BY year ORDER BY year
        """,
        conn,
    )
    yearly_volume_base = alt.Chart(yearly_volume).encode(x="year:O")
    yearly_volume_bars = yearly_volume_base.mark_bar(color="#3b82f6", opacity=0.8).encode(
        y="articles:Q", tooltip=["year", "articles"],
    )
    yearly_volume_avg = yearly_volume_base.mark_rule(color="#f97316", strokeDash=[6,3]).encode(
        y=alt.Y("average(articles):Q", title=""),
    )
    yearly_volume_chart = (
        (yearly_volume_bars + yearly_volume_avg)
        .properties(title="Articles par année", width=900, height=350)
    )
    yearly_volume_chart
    return


@app.cell
def _(alt, conn, pl):
    pub_hours = pl.read_database(
        """
        SELECT substr(published_at,12,2) as hour, COUNT(*) as articles
        FROM articles INDEXED BY idx_articles_published_at
        WHERE LENGTH(published_at) > 11
        GROUP BY hour ORDER BY hour
        """,
        conn,
    )
    pub_hours_chart = (
        alt.Chart(pub_hours)
        .mark_bar(color="#ec4899", cornerRadiusTopLeft=2, cornerRadiusTopRight=2)
        .encode(
            x=alt.X("hour:O", title="Heure"),
            y=alt.Y("articles:Q", title=""),
            tooltip=["hour", "articles"],
        )
        .properties(title="À quelle heure publie-t-on ?", width=900, height=300)
    )
    pub_hours_chart
    return


@app.cell
def _(alt, conn, pl):
    dow_data = pl.read_database(
        """
        SELECT CAST(strftime('%w', published_at) AS INTEGER) as dow, COUNT(*) as articles
        FROM articles GROUP BY dow ORDER BY dow
        """,
        conn,
    )
    jour_labels = ["Dimanche", "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi"]
    dow_data = dow_data.with_columns(
        pl.Series("jour", [jour_labels[d] for d in dow_data["dow"].to_list()])
    )
    dow_chart = (
        alt.Chart(dow_data)
        .mark_bar(color="#f59e0b", cornerRadiusTopLeft=2, cornerRadiusTopRight=2)
        .encode(
            x=alt.X("jour:N", sort=jour_labels, title=""),
            y=alt.Y("articles:Q", title=""),
            tooltip=["jour", "articles"],
        )
        .properties(title="Articles par jour de la semaine", width=600, height=300)
    )
    dow_chart
    return


@app.cell
def _(alt, conn, pl):
    heatmap_data = pl.read_database(
        """
        SELECT substr(published_at,1,4) as year,
               CAST(strftime('%j', published_at) AS INTEGER) as day_of_year,
               COUNT(*) as articles
        FROM articles INDEXED BY idx_articles_published_at
        WHERE published_at >= '2006-01-01'
        GROUP BY year, day_of_year
        """,
        conn,
    )
    heatmap_chart = (
        alt.Chart(heatmap_data)
        .mark_rect()
        .encode(
            x=alt.X("day_of_year:O", title="Jour de l'année", axis=alt.Axis(values=list(range(1, 366, 30)))),
            y=alt.Y("year:O", title=""),
            color=alt.Color("articles:Q", scale=alt.Scale(scheme="viridis"), legend=alt.Legend(title="Articles")),
            tooltip=["year", "day_of_year", "articles"],
        )
        .properties(title="Calendrier de publication — 20 ans en un coup d'œil", width=900, height=450)
    )
    heatmap_chart
    return


@app.cell
def _(mo):
    mo.md("""
    ---

    ## Ce dont le Québec parle

    Les rubriques révèlent les priorités d'une époque.
    L'Économie domine toujours, mais le paysage médiatique se transforme.
    """)
    return


@app.cell
def _(alt, conn, pl):
    top_sections = pl.read_database(
        """
        SELECT section, COUNT(*) as articles
        FROM articles WHERE section IS NOT NULL
        GROUP BY section ORDER BY articles DESC LIMIT 20
        """,
        conn,
    )
    top_sections_chart = (
        alt.Chart(top_sections)
        .mark_bar(color="#6366f1", cornerRadiusTopLeft=2, cornerRadiusTopRight=2)
        .encode(
            y=alt.Y("section:N", sort="-x", title=""),
            x=alt.X("articles:Q", title=""),
            tooltip=["section", "articles"],
        )
        .properties(title="Top 20 rubriques", width=900, height=500)
    )
    top_sections_chart
    return


@app.cell
def _(alt, conn, pl):
    section_year = pl.read_database(
        """
        SELECT section, substr(published_at,1,4) as year, COUNT(*) as articles
        FROM articles
        WHERE section IN (
            SELECT section FROM articles WHERE section IS NOT NULL
            GROUP BY section ORDER BY COUNT(*) DESC LIMIT 8
        )
        GROUP BY section, year ORDER BY section, year
        """,
        conn,
    )
    section_time_chart = (
        alt.Chart(section_year)
        .mark_area(opacity=0.7)
        .encode(
            x="year:O",
            y=alt.Y("articles:Q", stack="zero"),
            color=alt.Color("section:N", scale=alt.Scale(scheme="tableau20")),
            tooltip=["section", "year", "articles"],
        )
        .properties(title="L'évolution des grandes rubriques", width=900, height=400)
    )
    section_time_chart
    return


@app.cell
def _(alt, conn, pl):
    section_diversity = pl.read_database(
        """
        SELECT substr(published_at,1,4) as year,
               COUNT(DISTINCT section) as n_sections
        FROM articles INDEXED BY idx_articles_published_at
        WHERE section IS NOT NULL
        GROUP BY year ORDER BY year
        """,
        conn,
    )
    section_diversity_area = (
        alt.Chart(section_diversity)
        .mark_area(color="#f97316", opacity=0.3)
        .encode(x="year:O", y="n_sections:Q")
    )
    section_diversity_line = (
        alt.Chart(section_diversity)
        .mark_line(color="#ea580c", strokeWidth=2.5)
        .encode(
            x="year:O",
            y=alt.Y("n_sections:Q", title="Rubriques uniques"),
            tooltip=["year", "n_sections"],
        )
    )
    section_diversity_chart = (
        (section_diversity_area + section_diversity_line)
        .properties(title="La diversité des rubriques au fil des ans", width=900, height=350)
    )
    section_diversity_chart
    return


@app.cell
def _(mo):
    mo.md("""
    ---

    ## L'esprit du temps

    Certains mots explosent dans les manchettes, puis disparaissent.
    D'autres s'installent durablement.
    """)
    return


@app.cell
def _(alt, conn, pl):
    trends = pl.read_database(
        """
        SELECT 'COVID' as sujet, substr(a.published_at,1,4) as annee, COUNT(*) as n
        FROM articles_fts f JOIN articles a ON a.id = f.rowid
        WHERE f.articles_fts MATCH 'title:covid' GROUP BY annee
        UNION ALL
        SELECT 'Trump', substr(a.published_at,1,4), COUNT(*)
        FROM articles_fts f JOIN articles a ON a.id = f.rowid
        WHERE f.articles_fts MATCH 'title:trump' GROUP BY substr(a.published_at,1,4)
        UNION ALL
        SELECT 'Climat', substr(a.published_at,1,4), COUNT(*)
        FROM articles_fts f JOIN articles a ON a.id = f.rowid
        WHERE f.articles_fts MATCH 'title:climat' GROUP BY substr(a.published_at,1,4)
        UNION ALL
        SELECT 'Inflation', substr(a.published_at,1,4), COUNT(*)
        FROM articles_fts f JOIN articles a ON a.id = f.rowid
        WHERE f.articles_fts MATCH 'title:inflation' GROUP BY substr(a.published_at,1,4)
        UNION ALL
        SELECT 'Intelligence artificielle', substr(a.published_at,1,4), COUNT(*)
        FROM articles_fts f JOIN articles a ON a.id = f.rowid
        WHERE f.articles_fts MATCH 'title:intelligence title:artificielle' GROUP BY substr(a.published_at,1,4)
        ORDER BY sujet, annee
        """,
        conn,
    )
    trends_chart = (
        alt.Chart(trends)
        .mark_line(strokeWidth=2.5, point=True)
        .encode(
            x="annee:O",
            y=alt.Y("n:Q", title=""),
            color=alt.Color("sujet:N", scale=alt.Scale(range=["#ef4444", "#3b82f6", "#10b981", "#f59e0b", "#8b5cf6"])),
            tooltip=["sujet", "annee", "n"],
        )
        .properties(title="Sujets vedettes dans les manchettes", width=900, height=400)
    )
    trends_chart
    return


@app.cell
def _(mo):
    mo.md("""
    ### L'ère de l'intelligence artificielle

    Si l'IA n'apparaît qu'en pointillé dans le graphique précédent,
    c'est parce qu'elle explose sous plusieurs noms différents.
    """)
    return


@app.cell
def _(alt, conn, pl):
    ai_trends = pl.read_database(
        """
        SELECT 'Intelligence artificielle' as terme, substr(published_at,1,4) as annee, COUNT(*) as n
        FROM articles WHERE title LIKE '%intelligence artificielle%' GROUP BY annee
        UNION ALL
        SELECT 'IA', substr(published_at,1,4), COUNT(*)
        FROM articles WHERE title LIKE '% IA %' OR title LIKE 'IA %' OR title LIKE '% IA' OR title LIKE 'IA,' OR title LIKE 'IA :' GROUP BY substr(published_at,1,4)
        UNION ALL
        SELECT 'ChatGPT', substr(published_at,1,4), COUNT(*)
        FROM articles WHERE title LIKE '%ChatGPT%' OR title LIKE '%chatgpt%' GROUP BY substr(published_at,1,4)
        UNION ALL
        SELECT 'Algorithm%', substr(published_at,1,4), COUNT(*)
        FROM articles WHERE title LIKE '%algorithm%' GROUP BY substr(published_at,1,4)
        ORDER BY terme, annee
        """,
        conn,
    )
    ai_trends_chart = (
        alt.Chart(ai_trends)
        .mark_line(strokeWidth=2.5, point=True)
        .encode(
            x="annee:O",
            y=alt.Y("n:Q", title=""),
            color=alt.Color("terme:N", scale=alt.Scale(range=["#8b5cf6", "#06b6d4", "#ef4444", "#f59e0b"])),
            tooltip=["terme", "annee", "n"],
        )
        .properties(title="L'essor de l'IA dans les manchettes", width=900, height=400)
    )
    ai_trends_chart
    return


@app.cell
def _(mo):
    mo.md("""
    ### Les mots qui définissent chaque décennie

    Quels mots reviennent le plus souvent dans les titres ?
    La réponse change radicalement d'une décennie à l'autre.
    """)
    return


@app.cell
def _(conn, pl):
    import re as re_module

    STOPWORDS = {
        "de", "du", "le", "la", "les", "un", "une", "des", "et", "en", "à",
        "dans", "sur", "pour", "par", "avec", "au", "aux", "d", "l", "s",
        "se", "ce", "qui", "que", "ne", "pas", "son", "sa", "ses", "ont",
        "est", "sont", "elle", "il", "ils", "nous", "vous", "on", "y",
        "cette", "tout", "plus", "fait", "même", "aussi", "bien", "tres",
        "encore", "apres", "avant", "entre", "sous", "chez", "vers", "sans",
        "leurs", "leur", "ces", "été", "avoir", "etre", "mais", "ou", "donc",
        "ni", "car", "si", "que", "quoi", "dont", "quel", "quelle", "quels",
        "quelles", "comment", "pourquoi", "quand", "a", "the", "and", "of",
        "in", "to", "for", "new", "is", "it", "la", "au", "du", "le",
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "10",
    }

    decade_data = {}
    for decade_start, decade_label in [(2005, "2005–2009"), (2010, "2010–2014"), (2015, "2015–2019"), (2020, "2020–2026")]:
        decade_end = decade_start + 5 if decade_start < 2020 else 2027
        rows = pl.read_database(
            f"""
            SELECT title FROM articles INDEXED BY idx_articles_published_at
            WHERE title IS NOT NULL
              AND published_at >= '{decade_start}-01-01'
              AND published_at < '{decade_end}-01-01'
            LIMIT 200000
            """,
            conn,
        )
        word_counts = {}
        for title in rows["title"].to_list():
            for w in re_module.findall(r"[a-zàâäéèêëïîôùûüÿçœæ]{3,}", title.lower()):
                if w not in STOPWORDS and len(w) >= 3:
                    word_counts[w] = word_counts.get(w, 0) + 1
        top = sorted(word_counts.items(), key=lambda x: -x[1])[:20]
        decade_data[decade_label] = pl.DataFrame([{"mot": w, "n": c} for w, c in top])
    decade_word_frames = decade_data
    return (decade_word_frames,)


@app.cell
def _(alt, decade_word_frames):
    decade_charts = []
    for decade, df in sorted(decade_word_frames.items()):
        decade_charts.append(
            alt.Chart(df)
            .mark_bar(color="#6366f1", cornerRadiusTopLeft=2, cornerRadiusTopRight=2)
            .encode(
                y=alt.Y("mot:N", sort="-x", title=""),
                x=alt.X("n:Q", title=""),
                tooltip=["mot", "n"],
            )
            .properties(title=decade, width=220, height=300)
        )
    (decade_charts[0] | decade_charts[1] | decade_charts[2] | decade_charts[3])
    return


@app.cell
def _(mo):
    mo.md("""
    ---

    ## L'évolution des manchettes

    En 20 ans, les titres sont passés de ~45 à ~70 caractères en moyenne.
    Le web a tué la une de journal — et rendu les titres plus descriptifs.
    """)
    return


@app.cell
def _(alt, conn, pl):
    title_lengths = pl.read_database(
        """
        SELECT substr(published_at,1,4) as year,
               AVG(LENGTH(title)) as avg_chars
        FROM articles INDEXED BY idx_articles_published_at
        WHERE title IS NOT NULL
        GROUP BY year ORDER BY year
        """,
        conn,
    )
    title_len_area = (
        alt.Chart(title_lengths)
        .mark_area(color="#a78bfa", opacity=0.4)
        .encode(x="year:O", y="avg_chars:Q")
    )
    title_len_line = (
        alt.Chart(title_lengths)
        .mark_line(color="#7c3aed", strokeWidth=2.5)
        .encode(
            x="year:O",
            y=alt.Y("avg_chars:Q", title="Caractères (moyenne)"),
            tooltip=["year", alt.Tooltip("avg_chars:Q", format=".1f")],
        )
    )
    title_len_chart = (
        (title_len_area + title_len_line)
        .properties(title="Les titres s'allongent d'année en année", width=900, height=350)
    )
    title_len_chart
    return


@app.cell
def _(mo):
    mo.md("""
    ---

    ## La longueur des articles

    Les articles de *La Presse* sont-ils devenus plus courts ou plus longs ?
    Le passage au numérique a transformé les habitudes de lecture — et d'écriture.
    """)
    return


@app.cell
def _(alt, conn, pl):
    article_words_by_year = pl.read_database(
        """
        SELECT substr(published_at,1,4) as year,
               AVG(word_count) as avg_words
        FROM articles INDEXED BY idx_articles_published_at
        WHERE word_count IS NOT NULL
        GROUP BY year ORDER BY year
        """,
        conn,
    )
    article_words_area = (
        alt.Chart(article_words_by_year)
        .mark_area(color="#06b6d4", opacity=0.3)
        .encode(x="year:O", y="avg_words:Q")
    )
    article_words_line = (
        alt.Chart(article_words_by_year)
        .mark_line(color="#0891b2", strokeWidth=2.5)
        .encode(
            x="year:O",
            y=alt.Y("avg_words:Q", title="Mots (moyenne)"),
            tooltip=["year", alt.Tooltip("avg_words:Q", format=".0f")],
        )
    )
    article_words_chart = (
        (article_words_area + article_words_line)
        .properties(title="Longueur moyenne des articles par année (en mots)", width=900, height=350)
    )
    article_words_chart
    return


@app.cell
def _(alt, conn, pl):
    section_avg_words = pl.read_database(
        """
        SELECT section, AVG(word_count) as avg_words, COUNT(*) as n
        FROM articles
        WHERE section IS NOT NULL AND word_count IS NOT NULL
          AND section IN (
            SELECT section FROM articles WHERE section IS NOT NULL
            GROUP BY section ORDER BY COUNT(*) DESC LIMIT 15
          )
        GROUP BY section ORDER BY avg_words DESC
        """,
        conn,
    )
    section_words_chart = (
        alt.Chart(section_avg_words)
        .mark_bar(color="#0ea5e9", cornerRadiusTopLeft=2, cornerRadiusTopRight=2)
        .encode(
            y=alt.Y("section:N", sort="-x", title=""),
            x=alt.X("avg_words:Q", title="Mots (moyenne)"),
            tooltip=["section", alt.Tooltip("avg_words:Q", format=".0f"), "n"],
        )
        .properties(title="Rubriques les plus verbeuses (top 15)", width=900, height=450)
    )
    section_words_chart
    return


@app.cell
def _(alt, conn, pl):
    word_bins = pl.read_database(
        """
        SELECT
            CASE
                WHEN word_count < 200 THEN '< 200'
                WHEN word_count < 500 THEN '200–499'
                WHEN word_count < 1000 THEN '500–999'
                WHEN word_count < 2000 THEN '1000–1999'
                WHEN word_count < 5000 THEN '2000–4999'
                ELSE '5000+'
            END as bucket,
            COUNT(*) as articles
        FROM articles WHERE word_count IS NOT NULL AND word_count > 0
        GROUP BY bucket
        ORDER BY
            CASE bucket
                WHEN '< 200' THEN 1
                WHEN '200–499' THEN 2
                WHEN '500–999' THEN 3
                WHEN '1000–1999' THEN 4
                WHEN '2000–4999' THEN 5
                WHEN '5000+' THEN 6
            END
        """,
        conn,
    )
    bucket_order = ["< 200", "200–499", "500–999", "1000–1999", "2000–4999", "5000+"]
    word_dist_chart = (
        alt.Chart(word_bins)
        .mark_bar(color="#06b6d4", cornerRadiusTopLeft=2, cornerRadiusTopRight=2)
        .encode(
            x=alt.X("bucket:N", sort=bucket_order, title="Mots par article"),
            y=alt.Y("articles:Q", title=""),
            tooltip=["bucket", alt.Tooltip("articles:Q", format=",")],
        )
        .properties(title="Distribution de la longueur des articles", width=900, height=350)
    )
    word_dist_chart
    return


@app.cell
def _(mo):
    mo.md("""
    ---

    ## Les plumes de *La Presse*

    Derrière les millions d'articles, des milliers de journalistes.
    Voici les plus prolifiques — et les plus loquaces.
    """)
    return


@app.cell
def _(alt, conn, pl):
    top_authors = pl.read_database(
        """
        WITH ranked AS (
            SELECT TRIM(SUBSTR(author, 1, INSTR(author, CHAR(10))-1)) as author,
                   COUNT(*) as articles, AVG(word_count) as avg_words
            FROM articles WHERE author IS NOT NULL AND word_count IS NOT NULL
              AND INSTR(author, CHAR(10)) > 0
            GROUP BY author
            UNION ALL
            SELECT TRIM(author) as author,
                   COUNT(*) as articles, AVG(word_count) as avg_words
            FROM articles WHERE author IS NOT NULL AND word_count IS NOT NULL
              AND INSTR(author, CHAR(10)) = 0
            GROUP BY author
        )
        SELECT author, SUM(articles) as articles, AVG(avg_words) as avg_words
        FROM ranked
        WHERE author <> ''
          AND UPPER(author) NOT IN (
            'AGENCE FRANCE-PRESSE','LA PRESSE CANADIENNE','ASSOCIATED PRESS',
            'LA PRESSE','RELAXNEWS','CYBERPRESSE','AFP','SPORTCOM',
            'LE SOLEIL','AGENCE SCIENCE PRESSE','BLOOMBERG','REUTERS',
            'SILICON.FR','TECHNAUTE.CA','PRESSE CANADIENNE'
          )
          AND author NOT LIKE '%La Presse%'
          AND author NOT LIKE '%.com%' AND author NOT LIKE '%.fr%' AND author NOT LIKE '%.ca%'
        GROUP BY author ORDER BY articles DESC LIMIT 25
        """,
        conn,
    )
    authors_chart = (
        alt.Chart(top_authors)
        .mark_bar(color="#10b981", cornerRadiusTopLeft=2, cornerRadiusTopRight=2)
        .encode(
            y=alt.Y("author:N", sort="-x", title=""),
            x=alt.X("articles:Q", title=""),
            tooltip=["author", "articles", alt.Tooltip("avg_words:Q", format=".0f")],
        )
        .properties(title="Les 25 journalistes les plus prolifiques", width=900, height=550)
    )
    authors_chart
    return


@app.cell
def _(alt, conn, pl):
    author_verbosity = pl.read_database(
        """
        WITH ranked AS (
            SELECT TRIM(SUBSTR(author, 1, INSTR(author, CHAR(10))-1)) as author,
                   COUNT(*) as articles, AVG(word_count) as avg_words
            FROM articles WHERE author IS NOT NULL AND word_count IS NOT NULL
              AND INSTR(author, CHAR(10)) > 0
            GROUP BY author
            UNION ALL
            SELECT TRIM(author) as author,
                   COUNT(*) as articles, AVG(word_count) as avg_words
            FROM articles WHERE author IS NOT NULL AND word_count IS NOT NULL
              AND INSTR(author, CHAR(10)) = 0
            GROUP BY author
        )
        SELECT author, SUM(articles) as articles, AVG(avg_words) as avg_words
        FROM ranked
        WHERE author <> ''
          AND UPPER(author) NOT IN (
            'AGENCE FRANCE-PRESSE','LA PRESSE CANADIENNE','ASSOCIATED PRESS',
            'LA PRESSE','RELAXNEWS','CYBERPRESSE','AFP','SPORTCOM',
            'LE SOLEIL','AGENCE SCIENCE PRESSE','BLOOMBERG','REUTERS',
            'SILICON.FR','TECHNAUTE.CA','PRESSE CANADIENNE'
          )
          AND author NOT LIKE '%La Presse%'
          AND author NOT LIKE '%.com%' AND author NOT LIKE '%.fr%' AND author NOT LIKE '%.ca%'
        GROUP BY author
        HAVING SUM(articles) >= 500
        ORDER BY avg_words DESC LIMIT 25
        """,
        conn,
    )
    verbosity_chart = (
        alt.Chart(author_verbosity)
        .mark_bar(color="#f472b6", cornerRadiusTopLeft=2, cornerRadiusTopRight=2)
        .encode(
            y=alt.Y("author:N", sort="-x", title=""),
            x=alt.X("avg_words:Q", title="Mots par article (moyenne)"),
            tooltip=["author", alt.Tooltip("avg_words:Q", format=".0f"), "articles"],
        )
        .properties(title="Les 25 journalistes les plus loquaces (min. 500 articles)", width=900, height=550)
    )
    verbosity_chart
    return


@app.cell
def _(mo):
    mo.md("""
    ---

    ## Le dimanche, on lit plus

    La longueur des articles varie selon le jour de la semaine.
    Les dimanches, *La Presse* publie ses grands reportages.
    """)
    return


@app.cell
def _(alt, conn, pl):
    dow_length = pl.read_database(
        """
        SELECT CAST(strftime('%w', published_at) AS INTEGER) as dow,
               AVG(word_count) as avg_words
        FROM articles WHERE word_count IS NOT NULL
        GROUP BY dow ORDER BY dow
        """,
        conn,
    )
    jour_labels_2 = ["Dimanche", "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi"]
    dow_length = dow_length.with_columns(
        pl.Series("jour", [jour_labels_2[d] for d in dow_length["dow"].to_list()])
    )
    dow_length_chart = (
        alt.Chart(dow_length)
        .mark_bar(color="#c084fc", cornerRadiusTopLeft=2, cornerRadiusTopRight=2)
        .encode(
            x=alt.X("jour:N", sort=jour_labels_2, title=""),
            y=alt.Y("avg_words:Q", title="Mots (moyenne)"),
            tooltip=["jour", alt.Tooltip("avg_words:Q", format=".0f")],
        )
        .properties(title="Longueur moyenne des articles par jour de la semaine", width=600, height=300)
    )
    dow_length_chart
    return


@app.cell
def _(mo):
    mo.md("""
    ---

    ## Les mastodontes

    Certains articles dépassent les 10 000 mots — des reportages magistraux
    qui prennent le temps d'explorer en profondeur.
    """)
    return


@app.cell
def _(conn, mo, pl):
    longest_articles = pl.read_database(
        """
        SELECT title, section, author, published_at, word_count
        FROM articles WHERE word_count IS NOT NULL AND word_count > 0
        ORDER BY word_count DESC LIMIT 15
        """,
        conn,
    )
    longest_articles = longest_articles.with_columns(
        pl.col("published_at").str.slice(0, 10).alias("date"),
        pl.col("word_count").cast(pl.Int64).alias("mots"),
    )
    mo.ui.table(
        longest_articles.select("title", "section", "author", "date", "mots"),
        page_size=15,
    )
    return


@app.cell
def _(mo):
    mo.md("""
    ---

    ## Explorer les archives

    Recherchez un mot ou une phrase dans 20 ans de journalisme québécois.
    """)
    return


@app.cell
def _(mo):
    search_input = mo.ui.text(placeholder="Rechercher dans les archives...", full_width=True)
    search_input
    return (search_input,)


@app.cell
def _(conn, mo, pl, search_input):
    mo.stop(not search_input.value or len(search_input.value) < 3, "Tapez au moins 3 caractères")
    search_results = pl.read_database(
        """
        SELECT articles.title, articles.section, articles.author, articles.published_at
        FROM articles_fts JOIN articles ON articles_fts.rowid = articles.id
        WHERE articles_fts MATCH ?
        ORDER BY rank LIMIT 25
        """,
        conn,
        execute_options={"parameters": [search_input.value]},
    )
    mo.md(f"### Résultats pour « {search_input.value} » ({len(search_results)} résultats)")
    mo.ui.table(search_results, page_size=10)
    return


if __name__ == "__main__":
    app.run()
