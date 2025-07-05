# kobo-db-tools: Reclaim Your Kobo Reading Data

Welcome to `kobo-db-tools`, a project born from the frustration of limited access to personal reading data on Kobo devices and a commitment to empowering users with full control over their information.

## The Problem: Kobo and Data Ephemerality

As an avid Kobo reader, you might wonder about the fate of your reading data: how much time you've spent on a book, your page-turning habits, or how you adjust screen brightness. Kobo diligently collects this information within the `AnalyticsEvents` table of its `KoboReader.sqlite` database.

**However, a critical issue exists:** every time your Kobo connects to the internet (e.g., for syncing books or updates), the `AnalyticsEvents` table is **cleared** of its human-readable data. This means your valuable reading statistics are lost, becoming inaccessible for direct analysis.

The only remaining trace resides in the `Event` table, within the `ExtraData` field, where data is stored in an encrypted binary format (blob). Deciphering these blobs is a complex endeavor, not readily accessible to the average user.

**In essence: your data is present, but Kobo intentionally makes it challenging to retrieve once the device goes online.**

## The Solution: Empowering Data Ownership

`kobo-db-tools` offers a solution to this challenge, enabling you to extract and analyze your reading data before it is purged. There are two primary strategies to ensure you never lose your reading statistics again:

1.  **Offline Operation:** The simplest, though less practical, method. Avoid connecting your Kobo to the internet. While this prevents data deletion, it naturally limits synchronization and update functionalities.

2.  **Intelligent SQL Trigger (Recommended):** This is the more elegant and automated solution. You can implement an SQL trigger directly within your Kobo's `KoboReader.sqlite` database. This trigger will intercept and **prevent the deletion** operation on the `AnalyticsEvents` table. This ensures your valuable reading statistics are never purged from the database, maintaining a complete historical record even after online synchronizations.

    *Example SQL trigger (to be adapted and tested with caution on your device):*

    ```sql
    -- Create the trigger that copies data before deletion
    CREATE TRIGGER prevent_delete_on_analytics_events
    BEFORE DELETE ON AnalyticsEvents
    BEGIN
      SELECT RAISE(ABORT, 'Deletion is not allowed on AnalyticsEvents table');
    END;
    ```

    **Caution:** Modifying your Kobo's database carries inherent risks. Always back up your `KoboReader.sqlite` file before making any changes. This trigger is an example and may require specific adaptations for your Kobo's firmware version.

## Project Overview: What `kobo-db-tools` Does

This Rust project provides tools to:

*   **Parse the `KoboReader.sqlite` database:** Extract reading events, dictionary lookups, and brightness adjustments.
*   **Analyze Reading Sessions:** Calculate metrics such as reading time, pages turned, and more.
*   **Track Brightness Usage:** Analyze how and when you adjust screen brightness (both manual and natural light).

### How to Use

To use `kobo-db-tools` in your Rust project, add it as a dependency in your `Cargo.toml`:

```toml
[dependencies]
kobo-db-tools = "0.0.6" # Or the latest version
```

Then, you can parse a KoboReader.sqlite database and access the extracted data:

```rust
use kobo_db_tools::parser::{Parser, ParseOption};
use kobo_db_tools::export::{export_bookmarks, ExportFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = "path/to/your/KoboReader.sqlite"; // Replace with the actual path to your database

    // Parse all available data
    let analysis_all = Parser::parse_from_str(db_path, ParseOption::All)?;

    if let Some(sessions) = analysis_all.sessions {
        println!("Total reading sessions (All): {}", sessions.sessions_count());
    }
    if let Some(terms) = analysis_all.terms {
        println!("Total dictionary lookups (All): {}", terms.len());
    }
    if let Some(bookmarks) = analysis_all.bookmarks {
        println!("Total bookmarks (All): {}", bookmarks.len());
        // Example: Export bookmarks to a Markdown file
        let output_path = "bookmarks.md";
        export_bookmarks(&bookmarks, ExportFormat::Markdown, output_path)?;
        println!("Bookmarks exported to {}", output_path);
    }

    // Parse only reading sessions
    let analysis_sessions = Parser::parse_from_str(db_path, ParseOption::ReadingSessions)?;
    if let Some(sessions) = analysis_sessions.sessions {
        println!("Total reading sessions (only sessions): {}", sessions.sessions_count());
    }

    // Parse only dictionary lookups
    let analysis_terms = Parser::parse_from_str(db_path, ParseOption::DictionaryLookups)?;
    if let Some(terms) = analysis_terms.terms {
        println!("Total dictionary lookups (only terms): {}", terms.len());
    }

    Ok(())
}
```

### Future Enhancements and Analytical Perspectives

`kobo-db-tools` aims to evolve, offering more sophisticated analytical capabilities and data export options:

*   **Enhanced Statistical Analysis:** Beyond basic metrics, future versions will enable deeper insights, such as:
    *   Associating specific brightness adjustments with individual reading sessions.
    *   Linking dictionary lookups directly to the reading sessions in which they occurred.
    *   Aggregating and grouping all collected information by book, providing a comprehensive overview of your reading habits for each title.

*   **Flexible Data Export:** To maximize the utility of your data, the project plans to support various export formats, allowing you to use your reading statistics in other tools and applications:
    *   **JSON:** For easy integration with web applications or other programmatic uses.
    *   **CSV:** For spreadsheet analysis and compatibility with a wide range of data tools.
    *   **Markdown:** For human-readable summaries and reports.
    *   **SQLite:** To allow merging data from multiple Kobo devices or integrating with other SQLite-based databases.

*   **Multi-Device Data Merging:** A key objective is to facilitate the merging of reading data from multiple Kobo devices into a single, unified dataset, providing a holistic view of your reading across all your devices.

## Contributing

This project is in its early stages and welcomes contributions! If you have ideas for new features, improvements, or bug fixes, feel free to open an issue or a pull request. Before contributing, please review the contribution guidelines (to be defined).

## License

This project is released under the [MIT License](LICENSE).

---

We hope `kobo-db-tools` proves useful in exploring your reading data and reclaiming control over your digital information!