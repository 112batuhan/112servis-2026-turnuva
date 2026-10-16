-- Four more staff roles. Graphics designers and commentators are only listed on the
-- staff page and still play; pool testers and custom mappers can look at the map pool editor (every
-- stage, with the editor notes) but not change it, and don't play.
ALTER TYPE staff_role ADD VALUE 'graphics_designer';
ALTER TYPE staff_role ADD VALUE 'pool_tester';
ALTER TYPE staff_role ADD VALUE 'custom_mapper';
ALTER TYPE staff_role ADD VALUE 'commentator';
