use roead::aamp::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the Armor.baiprog test file
    let data = std::fs::read("test/aamp/AIProgram/Armor.baiprog")?;
    
    println!("Testing AAMP owned parser with Armor.baiprog ({} bytes)", data.len());
    
    // Parse using the existing owned API
    let pio = ParameterIO::from_binary(&data)?;
    
    // Verify basic structure
    println!("\nBasic structure verification:");
    println!("Version: {}", pio.version);
    println!("Data type: {}", pio.data_type);
    
    // Test access to root list
    println!("Root list has {} objects and {} lists", 
             pio.objects().len(), 
             pio.lists().len());
    
    // Show what objects exist
    println!("\nObjects at root level:");
    for (name, obj) in pio.objects().iter() {
        println!("  Object: {} ({} parameters)", name, obj.len());
        if obj.len() < 10 {  // Show parameters for small objects
            for (param_name, param) in obj.iter() {
                println!("    {}: {:?}", param_name, param);
            }
        }
    }
    
    println!("\nLists at root level:");
    for (name, list) in pio.lists().iter() {
        println!("  List: {} ({} objects, {} lists)", name, list.objects.len(), list.lists.len());
        
        // Show some detail for Action list
        if name.to_string() == "Action" {
            println!("    Action list details:");
            for (action_name, action_list) in list.lists.iter() {
                println!("      {}: {} objects, {} lists", action_name, action_list.objects.len(), action_list.lists.len());
                
                // Show Def object if it exists
                if let Some(def_obj) = action_list.objects.get("Def") {
                    for (param_name, param) in def_obj.iter() {
                        match param {
                            Parameter::StringRef(s) => println!("        {}: '{}'", param_name, s),
                            Parameter::String32(s) => println!("        {}: '{}'", param_name, s),
                            _ => println!("        {}: {:?}", param_name, param),
                        }
                    }
                }
                if action_list.objects.len() <= 3 {  // Don't spam too much
                    break;
                }
            }
        }
    }
    
    // Test DemoAIActionIdx object access
    println!("\nTesting DemoAIActionIdx object:");
    if let Some(demo_obj) = pio.object("DemoAIActionIdx") {
        println!("Found DemoAIActionIdx object with {} parameters", demo_obj.len());
        
        // Test a few specific parameters
        for (param_name, param) in demo_obj.iter().take(5) {
            println!("  {}: {:?}", param_name, param);
        }
    } else {
        println!("✗ DemoAIActionIdx object not found");
    }
    
    Ok(())
}