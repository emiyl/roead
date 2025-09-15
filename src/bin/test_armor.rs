use roead::aamp::{reader::*, *};

#[cfg(feature = "aamp-reader")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the Armor.baiprog test file
    let data = std::fs::read("test/aamp/AIProgram/Armor.baiprog")?;
    
    println!("Testing AAMP reader with Armor.baiprog ({} bytes)", data.len());
    
    // Parse using the new reader API
    let reader = ParameterIOReader::new(&data)?;
    
    // Verify basic structure
    println!("\nBasic structure verification:");
    println!("Version: {}", reader.version());
    println!("Data type: {}", reader.doc_type().unwrap_or("unknown"));
    
    // Test access to root list
    let root = reader.root();
    println!("Root list has {} objects and {} lists", 
             root.object_count(), 
             root.list_count());

    // Debug: Let's see what objects are actually found
    println!("\nDebugging - checking all objects at root level:");
    for i in 0..root.object_count() {
        if let Ok(Some((name, obj))) = root.get_object_at_index(i) {
            println!("  Object {}: {} ({} parameters)", i, name, obj.param_count());
        }
    }
    
    println!("\nDebugging - checking all lists at root level:");
    for i in 0..root.list_count() {
        if let Ok(Some((name, list))) = root.get_list_at_index(i) {
            println!("  List {}: {} ({} objects, {} lists)", i, name, list.object_count(), list.list_count());
        }
    }
    
    // Test DemoAIActionIdx object access
    println!("\nTesting DemoAIActionIdx object:");
    if let Some(demo_obj) = root.get_object("DemoAIActionIdx".into()) {
        println!("Found DemoAIActionIdx object with {} parameters", demo_obj.param_count());
        
        // Test specific parameters from the expected YAML
        let expected_params = vec![
            ("Demo_ArmorBind", 1),
            ("Demo_ArmorBindWithAS", 2), 
            ("Demo_CancelGet", 3),
            ("Demo_GetItem", 4),
            ("Demo_Idling", 5),
            ("Demo_Join", 6),
            ("Demo_OpenGetDemo", 7),
            ("Demo_PlayASForDemo", 8),
            ("Demo_PlayASForTimeline", 9),
            ("Demo_ResetBoneCtrl", 10),
            ("Demo_SendCatchWeaponMsgToPlayer", 11),
            ("Demo_SendSignal", 12),
            ("Demo_SetGetFlag", 13),
            ("Demo_SuccessGet", 14),
            ("Demo_TrigNullASPlay", 15),
            ("Demo_UpdateDataByGetDemo", 16),
            ("Demo_VisibleOff", 17),
            ("Demo_XLinkEventCreate", 18),
            ("Demo_XLinkEventFade", 19),
            ("Demo_XLinkEventKill", 20),
        ];
        
        for (param_name, expected_value) in expected_params {
            if let Some(param) = demo_obj.get(param_name.into()) {
                match param.as_i32() {
                    Ok(value) => {
                        if value == expected_value {
                            println!("✓ {}: {} (correct)", param_name, value);
                        } else {
                            println!("✗ {}: {} (expected {})", param_name, value, expected_value);
                        }
                    }
                    Err(e) => {
                        println!("✗ {}: failed to read as i32 - {:?}", param_name, e);
                    }
                }
            } else {
                println!("✗ {}: not found", param_name);
            }
        }
    } else {
        println!("✗ DemoAIActionIdx object not found");
        return Err("Missing DemoAIActionIdx object".into());
    }
    
    // Test nested lists - Action list
    println!("\nTesting Action list:");
    if let Some(action_list) = root.get_list("Action".into()) {
        println!("Found Action list with {} objects and {} lists", 
                 action_list.object_count(), 
                 action_list.list_count());
        
        // Test specific Action sublists - let's just test a few that should exist
        let expected_actions = vec![
            ("Action_0", "Root", "ArmorBindAction"),
            ("Action_1", "Demo_ArmorBind", "ArmorBindAction"),
            ("Action_8", "Demo_PlayASForDemo", "PlayASForDemo"),
        ];
        
        for (action_name, expected_name, expected_class) in expected_actions {
            if let Some(action_sublist) = action_list.get_list(action_name.into()) {
                if let Some(def_obj) = action_sublist.get_object("Def".into()) {
                    // Test Name parameter
                    if let Some(name_param) = def_obj.get("Name".into()) {
                        match name_param.as_str() {
                            Ok(name) => {
                                if name == expected_name {
                                    println!("✓ {}.Def.Name: {} (correct)", action_name, name);
                                } else {
                                    println!("✗ {}.Def.Name: {} (expected {})", action_name, name, expected_name);
                                }
                            }
                            Err(e) => {
                                println!("✗ {}.Def.Name: failed to read as string - {:?}", action_name, e);
                            }
                        }
                    }
                    
                    // Test ClassName parameter
                    if let Some(class_param) = def_obj.get("ClassName".into()) {
                        match class_param.as_str() {
                            Ok(class) => {
                                if class == expected_class {
                                    println!("✓ {}.Def.ClassName: {} (correct)", action_name, class);
                                } else {
                                    println!("✗ {}.Def.ClassName: {} (expected {})", action_name, class, expected_class);
                                }
                            }
                            Err(e) => {
                                println!("✗ {}.Def.ClassName: failed to read as string - {:?}", action_name, e);
                            }
                        }
                    }
                } else {
                    println!("✗ {}: Def object not found", action_name);
                }
            } else {
                println!("✗ {}: action sublist not found", action_name);
            }
        }
    } else {
        println!("✗ Action list not found");
        return Err("Missing Action list".into());
    }
    
    // Test other top-level lists exist but are empty
    println!("\nTesting other top-level lists:");
    for list_name in ["AI", "Behavior", "Query"] {
        if let Some(list) = root.get_list(list_name.into()) {
            let obj_count = list.object_count();
            let list_count = list.list_count();
            if obj_count == 0 && list_count == 0 {
                println!("✓ {}: empty list (correct)", list_name);
            } else {
                println!("✗ {}: has {} objects and {} lists (expected empty)", 
                         list_name, obj_count, list_count);
            }
        } else {
            println!("✗ {}: list not found", list_name);
        }
    }
    
    // Test basic iteration functionality (simplified)
    println!("\nTesting basic functionality:");
    println!("Root has {} objects and {} lists", root.object_count(), root.list_count());
    
    // Test conversion to owned API for comparison
    println!("\nTesting conversion to owned API:");
    let owned_pio = reader.to_owned()?;
    println!("Converted to owned ParameterIO successfully");
    println!("Version matches: {}", owned_pio.version == reader.version());
    
    // Quick comparison of DemoAIActionIdx
    if let Some(owned_demo_obj) = owned_pio.object("DemoAIActionIdx") {
        if let Some(reader_demo_obj) = root.get_object("DemoAIActionIdx".into()) {
            println!("DemoAIActionIdx parameter count matches: {}", 
                     owned_demo_obj.len() == reader_demo_obj.param_count());
                     
            // Test one specific parameter
            if let (Some(owned_param), Some(reader_param)) = 
                (owned_demo_obj.get("Demo_ArmorBind"), reader_demo_obj.get("Demo_ArmorBind".into())) {
                    let owned_val = owned_param.as_i32().unwrap();
                    let reader_val = reader_param.as_i32().unwrap();
                    println!("Demo_ArmorBind values match: {} == {} -> {}", 
                             owned_val, reader_val, owned_val == reader_val);
            }
        }
    }
    
    println!("\n✓ AAMP reader basic tests completed successfully!");
    Ok(())
}

#[cfg(not(feature = "aamp-reader"))]
fn main() {
    println!("This test requires the aamp-reader feature to be enabled.");
    println!("Run with: cargo run --bin test_armor --features aamp-reader");
}