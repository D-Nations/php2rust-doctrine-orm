//! The `taxi` model set and its constructors.
//!
//! Upstream: doctrine/orm tests/Tests/Models/Taxi/Car.php tests/Tests/Models/Taxi/Driver.php tests/Tests/Models/Taxi/Ride.php tests/Tests/Models/Taxi/PaidRide.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]

use php_value::Object;

use crate::orm::{new_object, s, set, set_obj};

pub const CAR: &str = "Doctrine\\Tests\\Models\\Taxi\\Car";
pub const DRIVER: &str = "Doctrine\\Tests\\Models\\Taxi\\Driver";
pub const RIDE: &str = "Doctrine\\Tests\\Models\\Taxi\\Ride";
pub const PAID_RIDE: &str = "Doctrine\\Tests\\Models\\Taxi\\PaidRide";

/// `useModelSet('taxi')`.
pub const TAXI_MODELS: [&str; 4] = [PAID_RIDE, RIDE, CAR, DRIVER];

/// `new Car()` with `setBrand()` and `setModel()`.
pub fn car(brand: &str, model: &str) -> Object {
    let car = new_object(CAR);
    set(&car, "brand", s(brand));
    set(&car, "model", s(model));
    car
}

/// `new Driver()` with `setName()`.
pub fn driver(name: &str) -> Object {
    let driver = new_object(DRIVER);
    set(&driver, "name", s(name));
    driver
}

/// `new Ride($driver, $car)` or `new PaidRide($driver, $car)`.
pub fn ride(class: &str, driver: &Object, car: &Object) -> Object {
    let ride = new_object(class);
    set_obj(&ride, "driver", driver);
    set_obj(&ride, "car", car);
    ride
}
