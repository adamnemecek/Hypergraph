use {
    crate::category::{Composable, ComposableMutating, HasIdentity},
    std::fmt::Debug,
};

pub trait Monoidal {
    /*
    change the morphism self to the morphism (self \otimes other)
    */
    fn monoidal(&mut self, other: Self);
}

#[derive(PartialEq, Eq, Clone)]
pub struct GenericMonoidalMorphismLayer<BoxType, Lambda: Eq + Copy> {
    /*
    a single layer for a black box filled morphism
    in a monoidal category whose objects
        are presented as tensor products of Lambda
    the black boxes are labelled with BoxType
    */
    pub blocks: Vec<BoxType>,
    pub left_type: Vec<Lambda>,
    pub right_type: Vec<Lambda>,
}

impl<BoxType, Lambda> GenericMonoidalMorphismLayer<BoxType, Lambda>
where
    Lambda: Eq + Copy,
{
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            blocks: vec![],
            left_type: vec![],
            right_type: vec![],
        }
    }
}

impl<BoxType, Lambda> HasIdentity<Vec<Lambda>> for GenericMonoidalMorphismLayer<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: HasIdentity<Lambda>,
{
    fn identity(on_type: &Vec<Lambda>) -> Self {
        let mut answer = Self::new();
        for cur_type in on_type {
            answer.blocks.push(BoxType::identity(cur_type));
            answer.left_type.push(*cur_type);
            answer.right_type.push(*cur_type);
        }
        answer
    }
}

impl<BoxType, Lambda> Monoidal for GenericMonoidalMorphismLayer<BoxType, Lambda>
where
    Lambda: Eq + Copy,
{
    fn monoidal(&mut self, other: Self) {
        self.blocks.extend(other.blocks);
        self.left_type.extend(other.left_type);
        self.right_type.extend(other.right_type);
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct GenericMonoidalMorphism<BoxType, Lambda: Eq + Copy> {
    /*
    a black box filled morphism
    in a monoidal category whose objects
        are presented as tensor products of Lambda
    the black boxes are labelled with BoxType
    when given a function from BoxType to the
        actual type for the morphisms in the desired category
        one can interpret this as the aforementioned type
        by building up with composition and monoidal
    */
    layers: Vec<GenericMonoidalMorphismLayer<BoxType, Lambda>>,
}

impl<Lambda, BoxType> GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy,
{
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { layers: vec![] }
    }

    #[allow(dead_code)]
    pub fn depth(&self) -> usize {
        self.layers.len()
    }

    #[allow(dead_code)]
    fn append_layer(
        &mut self,
        next_layer: GenericMonoidalMorphismLayer<BoxType, Lambda>,
    ) -> Result<(), String> {
        if let Some(last_so_far) = self.layers.pop() {
            if last_so_far.right_type != next_layer.left_type {
                return Err("type mismatch in morphims composition".to_string());
            }
            self.layers.push(last_so_far);
        }
        self.layers.push(next_layer);
        Ok(())
    }
}

impl<Lambda, BoxType> HasIdentity<Vec<Lambda>> for GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy,
    BoxType: HasIdentity<Lambda>,
{
    #[allow(dead_code)]
    fn identity(on_this: &Vec<Lambda>) -> Self {
        Self {
            layers: vec![<_>::identity(on_this)],
        }
    }
}

impl<Lambda, BoxType> Monoidal for GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy + Debug,
    BoxType: Clone + HasIdentity<Lambda>,
{
    fn monoidal(&mut self, other: Self) {
        let self_len = self.layers.len();
        let others_len = other.layers.len();
        let mut last_other_type: Vec<Lambda> = vec![];
        let mut last_self_type: Vec<Lambda> = vec![];
        for (n, cur_self_layer) in self.layers.iter_mut().enumerate() {
            last_self_type = cur_self_layer.right_type.clone();
            cur_self_layer.monoidal(if n < other.layers.len() {
                last_other_type = other.layers[n].right_type.clone();
                other.layers[n].clone()
            } else {
                <_>::identity(&last_other_type)
            })
        }
        for n in self_len..others_len {
            let mut new_layer = GenericMonoidalMorphismLayer::identity(&last_self_type);
            new_layer.monoidal(other.layers[n].clone());
            let _ = self.append_layer(new_layer);
        }
    }
}

fn layers_composable<Lambda: Eq + Copy + Debug, BoxType>(
    l: &[GenericMonoidalMorphismLayer<BoxType, Lambda>],
    r: &[GenericMonoidalMorphismLayer<BoxType, Lambda>],
) -> Result<(), String> {
    if l.is_empty() || r.is_empty() {
        if l.is_empty() && r.is_empty() {
            return Ok(());
        } else if l.is_empty() {
            let other_interface = &r[0].left_type;
            if other_interface.is_empty() {
                return Ok(());
            } else {
                return Err("Mismatch in cardinalities of common interface".to_string());
            }
        } else {
            let self_interface = &l.last().unwrap().right_type;
            if self_interface.is_empty() {
                return Ok(());
            } else {
                return Err("Mismatch in cardinalities of common interface".to_string());
            }
        }
    }
    let lhs = &l.last().unwrap().right_type;
    let rhs = &r[0].left_type;
    if lhs.len() != rhs.len() {
        return Err("Mismatch in cardinalities of common interface".to_string());
    }
    if lhs == rhs {
        return Ok(());
    }
    if let Some((w1, w2)) = lhs.iter().zip(rhs.iter()).find(|(w1, w2)| w1 != w2) {
        return Err(format!(
            "Mismatch in labels of common interface. At some index there was {:?} vs {:?}",
            w1, w2
        ));
    }

    Err("Mismatch in labels of common interface at some unknown index.".to_string())
}

impl<Lambda, BoxType> ComposableMutating<Vec<Lambda>> for GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy + Debug,
{
    fn composable(&self, other: &Self) -> Result<(), String> {
        layers_composable(&self.layers, &other.layers)
    }

    fn compose(&mut self, other: Self) -> Result<(), String> {
        for next_layer in other.layers {
            self.append_layer(next_layer)?;
        }
        Ok(())
    }

    fn domain(&self) -> Vec<Lambda> {
        self.layers
            .first()
            .map(|x| x.left_type.clone())
            .unwrap_or_default()
    }

    fn codomain(&self) -> Vec<Lambda> {
        self.layers
            .last()
            .map(|x| x.right_type.clone())
            .unwrap_or_default()
    }
}

pub trait MonoidalMorphism<T: Eq>: Monoidal + Composable<T> {}
pub trait MonoidalMutatingMorphism<T: Eq>: Monoidal + ComposableMutating<T> {}

pub trait GenericMonoidalInterpretableMut<Lambda: Eq + Copy + Debug>:
    Monoidal + ComposableMutating<Vec<Lambda>> + HasIdentity<Vec<Lambda>>
{
    /*
    given a function from BoxType to the
        actual type (Self) for the morphisms in the desired category
        one can interpret a GenericaMonoidalMorphism as a Self
        by building up with composition and monoidal
    */
    fn interpret<F, BoxType>(
        morphism: &GenericMonoidalMorphism<BoxType, Lambda>,
        black_box_interpreter: &F,
    ) -> Result<Self, String>
    where
        F: Fn(&BoxType) -> Result<Self, String>,
    {
        let mut answer = Self::identity(&morphism.domain());
        for layer in &morphism.layers {
            let Some(first) = &layer.blocks.first() else {
                return Err("somehow an empty layer in a generica monoidal morphism???".to_string());
            };
            let mut cur_layer = black_box_interpreter(first)?;
            for block in &layer.blocks[1..] {
                cur_layer.monoidal(black_box_interpreter(block)?);
            }
            answer.compose(cur_layer)?;
        }
        Ok(answer)
    }
}
pub trait GenericMonoidalInterpretable<Lambda: Eq + Copy + Debug>:
    Monoidal + Composable<Vec<Lambda>> + HasIdentity<Vec<Lambda>>
{
    /*
    given a function from BoxType to the
        actual type (Self) for the morphisms in the desired category
        one can interpret a GenericaMonoidalMorphism as a Self
        by building up with composition and monoidal
    only different from above because of the distinction between compositions
        that are done by modifying self to the composition self;other
        or that return a new self;other
    */
    fn interpret<F, BoxType>(
        morphism: &GenericMonoidalMorphism<BoxType, Lambda>,
        black_box_interpreter: &F,
    ) -> Result<Self, String>
    where
        F: Fn(&BoxType) -> Result<Self, String>,
    {
        let mut answer = Self::identity(&morphism.domain());
        for layer in &morphism.layers {
            let Some(first) = &layer.blocks.first() else {
                return Err("somehow an empty layer in a generica monoidal morphism???".to_string());
            };
            let mut cur_layer = black_box_interpreter(first)?;
            for block in &layer.blocks[1..] {
                cur_layer.monoidal(black_box_interpreter(block)?);
            }
            answer = answer.compose(&cur_layer)?;
        }
        Ok(answer)
    }
}

impl<Lambda, BoxType> MonoidalMutatingMorphism<Vec<Lambda>>
    for GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy + Debug,
    BoxType: HasIdentity<Lambda> + Clone,
{
    /*
    the most obvious implementation of MonoidalMutatingMorphism is GenericMonoidalMorphism itself
    use all the structure of monoidal, compose, identity provided by concatenating blocks and layers appropriately
    */
}

impl<Lambda, BoxType> GenericMonoidalInterpretableMut<Lambda>
    for GenericMonoidalMorphism<BoxType, Lambda>
where
    Lambda: Eq + Copy + Debug,
    BoxType: HasIdentity<Lambda> + Clone,
{
    /*
    the most obvious implementation of GenericMonoidalInterpretableMut is GenericMonoidalMorphism itself
    use the default implementation given in the trait itself
    in Frobenius we override the default implementation with just a clone
        because there we are only concerned with the case when black_box_interpreter
        was just sending the black boxes with the same sort of black box
    */
}
