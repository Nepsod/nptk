use nptk_core::app::font_ctx::FontContext;

fn main() {
    println!("Testing font system...");
    
    // Create a font context with system fonts
    let mut font_ctx = FontContext::new_with_system_fonts();
    
    // Test unicode character selection
    let test_chars = ['≠', '←', '↓', '→', '≤', '≥', 'A', '中', '🌍'];
    
    for ch in test_chars {
        if let Some(font) = font_ctx.select_for_char(ch) {
            println!("✓ Character '{}' found font: {:?}", ch, font.family_name());
        } else {
            println!("✗ Character '{}' - no font found", ch);
        }
    }
    
    // Test parley font context creation
    let parley_font_cx = font_ctx.create_parley_font_context();
    println!("✓ Successfully created parley FontContext");
    
    println!("Font system test completed!");
}
