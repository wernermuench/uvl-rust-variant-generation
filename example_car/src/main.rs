use uvl_macros::{feat_value, feat_if, feat_ifdef, feat};


// Calculates the total cost of the vehicle based on the selected features
fn calculate_total_cost() -> f64 {
    // Base Price
    let base_price = feat_value!("BasePrice");
    let mut total_cost = base_price;

    println!("Base Price: \t\t\t\t{:.2} €", base_price);

    // Engine Costs
    feat_if!("Electric", {
        let price = feat_value!("Electric.Price");
        total_cost += price;
        println!(" + Electric Engine: \t\t\t{:.2} €", price);
    });

    feat_if!("Gasoline", {
        let price = feat_value!("Gasoline.Price");
        total_cost += price;
        println!(" + Gasoline Engine: \t\t\t{:.2} €", price);
    });

    // Battery Costs
    feat_if!("Battery", {
        let battery_cost = 
            feat_if!("HybridBattery", {
                feat_value!("HybridBattery.Price")
            }
            else {
                feat_if!("LongRange", { 
                    feat_value!("LongRange.Price") 
                } 
                else { 
                    feat_if!("NormalRange", {
                        feat_value!("NormalRange.Price") 
                    } 
                    else {
                        0.0
                    })
                })
            });

        if battery_cost > 0.0 {
            total_cost += battery_cost;
            println!(" + Battery Upgrade: \t\t\t{:.2} €", battery_cost);
        }
    });

    // Paint Costs
    feat_if!("Blue", { 
        let price = feat_value!("Blue.Price");
        total_cost += price;
        println!(" + Paint (Blue): \t\t\t{:.2} €", price);
    });
    
    feat_if!("White", { 
        let price = feat_value!("White.Price");
        total_cost += price; 
        println!(" + Paint (White): \t\t\t{:.2} €", price);
    });
    
    feat_if!("Black", { 
        let price = feat_value!("Black.Price");
        total_cost += price; 
        println!(" + Paint (Black): \t\t\t{:.2} €", price);
    });

    // Assistance Systems
    feat_if!("Autopilot", {
        let price = feat_value!("Autopilot.Price");
        total_cost += price;
        println!(" + Autopilot System: \t\t\t{:.2} €", price);
    });

    // Cruise Control
    feat_if!("Cruise_Control", {
        let price = feat_value!("Cruise_Control.Price");
        total_cost += price;
        println!(" + Cruise Control: \t\t\t{:.2} €", price);
    });

    // Sensor Costs 
    feat_if!("UltrasonicSensor > 0", {
        let count = feat_value!("UltrasonicSensor");
        // Assume a fixed unit price here
        let unit_price = 1000.0; 
        let sensor_total = (count as f64) * unit_price;
        
        total_cost += sensor_total;
        println!(" + Sensors ({} units, {:.2} €/unit): \t{:.2} €", count, unit_price, sensor_total);
    });

    total_cost
}


#[feat("Autopilot && Electric")]
fn activate_autonomous_driving() {
    println!("Autopilot mode is activated!");
}


fn main() {

    println!("--- UVL Car Configurator ---");

    // Feature values
    let manufacturer   = feat_value!("Manufacturer"); // String
    let model_name     = feat_value!("ModelName");    // String
    let base_price      = feat_value!("BasePrice");    // Real
    
    println!("Vehicle: {} {} (Base Price: ${})", manufacturer, model_name, base_price);


    // Boolean features with attributes
    feat_if!("Electric", {
        println!("Engine Type: Electric");
        let power = feat_value!("Electric.PS");
        println!("Performance: {} HP", power);
    });

    feat_if!("Gasoline", {
        println!("Engine Type: Gasoline");
        let power = feat_value!("Gasoline.PS");
        println!("Performance: {} HP", power);
    });


    // Nested if else constructs
    feat_if!("Battery", {
        feat_if!("LongRange", {
            let cap = feat_value!("LongRange.Capacity");
            println!("Battery Pack: Long Range ({} kWh)", cap);
        } else {
            println!("Battery Pack: Standard Range or Hybrid Battery");
        });
    });


    // Feature cardinality
    feat_if!("UltrasonicSensor > 0", {
        let count = feat_value!("UltrasonicSensor");
        let sensor_unit_price =  1000; //= feat_value!("UltrasonicSensor.Price");
        println!("Sensors: {} units detected. Total sensor cost: ${}", count, count * sensor_unit_price);
        
        if count == 8 {
            println!("Full 360-degree sensor coverage active!");
        }
    });


    // Multi-word features
    feat_if!("Cruise_Control", {
        println!("Feature Active: Adaptive Cruise Control.");
    });


    // Testing conditional function call
    feat_ifdef!("Autopilot", {
        activate_autonomous_driving();
    } else {
        println!("Autopilot is not available!");
    });
    

    // Testing UVL constraints
    feat_if!("Battery && Gasoline && !Electric", {
        println!("ERROR: Only an electric car can have a battery!");
    });
    feat_if!("Autopilot && !Electric", {
        println!("ERROR: Autopilot selected for a non-electric vehicle!");
    });

    feat_if!("Manufacturer == \"CrazyCompany\" && Electric && Electric.PS > 1000", {
        println!("Crazy electric car configured!");
    });

    
    // Testing macros within expressions
    let total = base_price + feat_ifdef!("Electric", { feat_value!("Electric.Price") } 
                                                     else { feat_value!("Gasoline.Price") });
    println!("Current Price Calculation (Base + Engine): ${}", total);


    // Testing macro functions: sel()
    feat_if!("(sel(Cruise_Control) + sel(Autopilot) + sel(ParkAssist)) == 0 \
           || (sel(Cruise_Control) + sel(Autopilot) + sel(ParkAssist)) <= 2", {

        println!("Group cardinality satisfied!");

    } else {

        println!("Group cardinality is not satisfied!");

    });

    
    println!("\n\n------------ Cost Calculation Service ------------");

    let final_price = calculate_total_cost();

    println!("--------------------------------------------------");
    println!("Total Vehicle Configuration Cost: \t{:.2} €", final_price);

}
