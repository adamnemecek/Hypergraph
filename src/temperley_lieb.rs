use {
    crate::{
        category::{Composable, HasIdentity},
        linear_combination::LinearCombination,
        monoidal::{Monoidal, MonoidalMorphism},
    },
    itertools::Itertools,
    num::{One, Zero},
    petgraph::{
        algo::{connected_components, has_path_connecting, DfsSpace},
        Graph, Undirected,
    },
    std::{
        collections::HashSet,
        fmt::Debug,
        hash::Hash,
        ops::{Add, AddAssign, Mul, MulAssign},
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Pair(pub usize, pub usize);

impl Pair {
    pub fn iter(&self) -> impl Iterator<Item = usize> {
        [self.0, self.1].into_iter()
    }

    pub fn map(&self, f: impl Fn(usize) -> usize) -> Self {
        Self(f(self.0), f(self.1))
    }

    pub fn all(&self, f: impl Fn(usize) -> bool) -> bool {
        f(self.0) && f(self.1)
    }

    pub fn any(&self, f: impl Fn(usize) -> bool) -> bool {
        f(self.0) || f(self.1)
    }

    fn flip_upside_down(&self, source: usize, target: usize) -> Self {
        self.map(|v| if v < source { v + target } else { v - source })
    }

    pub fn sort(&self) -> Self {
        Self::sorted(self.0, self.1)
    }

    pub fn sorted(x: usize, y: usize) -> Self {
        if x < y {
            Self(x, y)
        } else {
            Self(y, x)
        }
    }

    pub fn contains(&self, x: usize) -> bool {
        (x < self.0 && x > self.1) || (x < self.1 && x > self.0)
    }
}

impl From<(usize, usize)> for Pair {
    fn from(value: (usize, usize)) -> Self {
        Self(value.0, value.1)
    }
}

#[derive(PartialEq, Eq, Clone, Hash, Debug)]
struct PerfectMatching {
    /*
    if gather all the entries in all the pairs, this gives 0..2n-1
    with n being the length of pairs
    each entry of pairs then says those two numbers are matched
    */
    pairs: Vec<Pair>,
}

impl FromIterator<Pair> for PerfectMatching {
    /*
    build a PerfectMatching from something that iterates to yield Pair
    makes sure that this iterator gives all the numbers 0..2n-1
    */
    fn from_iter<T: IntoIterator<Item = Pair>>(pair_prime: T) -> Self {
        let pairs: Vec<Pair> = pair_prime.into_iter().collect();
        let max_expected = pairs.len() * 2;
        let seen: HashSet<_> = pairs
            .iter()
            .flat_map(|x| {
                assert!(x.all(|x| x < max_expected));
                x.iter()
            })
            .collect();
        assert_eq!(seen.len(), max_expected);
        let mut ret_val = Self { pairs };

        ret_val.canonicalize();
        ret_val
    }
}

impl From<Vec<Pair>> for PerfectMatching {
    fn from(value: Vec<Pair>) -> Self {
        Self::from_iter(value)
    }
}

impl PerfectMatching {
    #[allow(dead_code)]
    fn new(pair_prime: &[Pair]) -> Self {
        Self::from_iter(pair_prime.iter().cloned())
    }

    pub fn shift_index(&self, threshold: usize, shift_amount: usize) -> Self {
        self.pairs
            .iter()
            .map(|p| p.map(|v| if v >= threshold { v + shift_amount } else { v }))
            .collect()
    }

    fn canonicalize(&mut self) {
        /*
        each matched pair is given as the smaller number first
        */
        for p in self.pairs.iter_mut() {
            *p = p.sort();
        }

        self.pairs.sort();
    }

    fn flip_upside_down(&self, source: usize, target: usize) -> Self {
        /*
        interpret the numbers 0..source as on the one layer
        then continuing the numbering on the other layer
        so this is a perfect matching as in a BrauerDiagram
        with specified source and target, then what is the perfect matching from
        flipping the diagram and still using the same convention for how the nodes are numbered
        */
        self.pairs
            .iter()
            .map(|x| x.flip_upside_down(source, target))
            .collect()
    }

    fn non_crossing(&self, source: usize, _target: usize) -> bool {
        /*
        when interpreting this as a BrauerDiagram with specified domain/codomain (sum of both=2n)
        is it actually temperley lieb with no crossings
        */
        let source_lines = self.pairs.iter().filter(|p| p.all(|x| x < source)).cloned();
        // source_lines are the lines connecting two points both on source side
        let source_crossing_tests = source_lines.clone().combinations(2);
        for cur_item in source_crossing_tests {
            let first_block = cur_item[0];
            let second_block = cur_item[1];
            let a = first_block.contains(second_block.0);
            let b = first_block.contains(second_block.1);
            if a != b {
                // a pair of lines that both connected source dots, crossed
                return false;
            }
        }
        // no crossing lines can use these indices because they are blocked by a line connecting
        //      two source points
        let mut no_through_lines_idx: HashSet<_> = source_lines
            .flat_map(|Pair(x, y)| (1 + x.min(y))..x.max(y))
            .collect();

        // the lines connecting two points both on target side
        let target_lines = self
            .pairs
            .iter()
            .filter(|p| p.all(|x| x >= source))
            .cloned();
        let target_crossing_tests = target_lines.clone().combinations(2);
        for cur_item in target_crossing_tests {
            let first_block = cur_item[0];
            let second_block = cur_item[1];
            let a = first_block.contains(second_block.0);
            let b = first_block.contains(second_block.1);
            if a != b {
                // a pair of lines that both connected target dots, crossed
                return false;
            }
        }

        // no crossing lines can use these indices because they are blocked by a line connecting
        // two target points

        no_through_lines_idx.extend(target_lines.flat_map(|Pair(x, y)| (1 + x.min(y))..x.max(y)));

        // now check that those crossing lines don't use those indices that were stated to be forbidden
        let through_lines = self
            .pairs
            .iter()
            .filter(|Pair(z, w)| (*z < source && *w >= source) || (*w < source && *z >= source))
            .map(|p| p.sort());

        if through_lines
            .clone()
            .any(|p| p.any(|x| no_through_lines_idx.contains(&x)))
        {
            return false;
        }

        // the induced map from the through_lines is monotonically increasing
        through_lines.map(|Pair(_, w)| w).is_sorted()
    }
}

/*
a single Brauer Diagram (and an accompanying power of delta)
- a domain size
- a codomain size
- some number of circles contributing delta^k
- a perfect matching on domain+codomain
*/
#[derive(PartialEq, Eq, Hash, Clone)]
struct ExtendedPerfectMatching((usize, usize, usize, PerfectMatching));

impl Mul for ExtendedPerfectMatching {
    /*
    concatenate the two diagrams
    removing any circles, and adding them to the combined power of delta
    */
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let (self_dom, self_cod, self_delta_pow, self_diagram) = self.0;
        let (rhs_dom, rhs_cod, rhs_delta_pow, rhs_diagram) = rhs.0;
        assert_eq!(rhs_dom, self_cod);
        let mut g = Graph::<(), (), Undirected>::new_undirected();
        let mut node_idcs = vec![None; self_dom + self_cod + rhs_cod];
        let self_pairs_copy = self_diagram.pairs.clone();
        for Pair(p, q) in self_diagram.pairs {
            let p_loc = g.add_node(());
            node_idcs[p] = Some(p_loc);
            let q_loc = g.add_node(());
            node_idcs[q] = Some(q_loc);
            g.add_edge(p_loc, q_loc, ());
        }
        for (idx, cur_item) in node_idcs.iter().enumerate().take(self_dom + self_cod) {
            assert!(
                cur_item.is_some(),
                "index for {idx} unset. These were the ones in self_diagram {:?}",
                self_pairs_copy
            );
        }
        let rhs_pairs_copy = rhs_diagram.pairs.clone();
        for Pair(p, q) in rhs_diagram.pairs {
            let p_loc = if p >= rhs_dom {
                let p_loc_temp = g.add_node(());
                node_idcs[p + self_dom] = Some(p_loc_temp);
                p_loc_temp
            } else {
                node_idcs[p + self_dom].unwrap()
            };
            let q_loc = if q >= rhs_dom {
                let q_loc_temp = g.add_node(());
                node_idcs[q + self_dom] = Some(q_loc_temp);
                q_loc_temp
            } else {
                node_idcs[q + self_dom].unwrap()
            };
            g.add_edge(p_loc, q_loc, ());
        }
        for (idx, cur_item) in node_idcs.iter().enumerate() {
            assert!(
                cur_item.is_some(),
                "index for {idx} unset. These were the ones in rhs {:?}",
                rhs_pairs_copy
            );
        }
        let endpoints = self_dom + rhs_cod;
        let mut endpoints_done = HashSet::<usize>::with_capacity(endpoints);
        let mut workspace = DfsSpace::new(&g);
        let mut final_matching = Vec::with_capacity(endpoints / 2);
        for i in 0..endpoints {
            if endpoints_done.contains(&i) {
                continue;
            }
            let i_loc = node_idcs[if i < self_dom { i } else { i + self_cod }].unwrap();
            for j in (i + 1)..endpoints {
                let j_loc = node_idcs[if j < self_dom { j } else { j + self_cod }].unwrap();
                let ij_conn = has_path_connecting(&g, i_loc, j_loc, Some(&mut workspace));
                if ij_conn {
                    final_matching.push(Pair(i, j));
                    endpoints_done.insert(i);
                    endpoints_done.insert(j);
                    break;
                }
            }
        }
        let new_delta_power =
            connected_components(&g) + self_delta_pow + rhs_delta_pow - (endpoints / 2);
        Self((self_dom, rhs_cod, new_delta_power, final_matching.into()))
    }
}

#[derive(Clone)]
pub struct BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy,
{
    /*
    a linear combination of (usize,PerfectMatching)
        where a term (k,match) means \delta^k*(match interpreted with source and target)
    the source and target are common for all terms
    is_def_tl means all the terms are non-crossing
    */
    diagram: LinearCombination<T, (usize, PerfectMatching)>,
    source: usize,
    target: usize,
    is_def_tl: bool,
}

impl<T> PartialEq for BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.diagram == other.diagram && self.source == other.source && self.target == other.target
    }
}

impl<T> Debug for BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrauerMorphism")
            .field("diagram", &self.diagram)
            .field("source", &self.source)
            .field("target", &self.target)
            .field("is_def_tl", &self.is_def_tl)
            .finish()
    }
}

impl<T> HasIdentity<usize> for BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy,
{
    fn identity(on_this: &usize) -> Self {
        let matching: PerfectMatching = (0..*on_this).map(|x| Pair(x, x + on_this)).collect();
        Self {
            diagram: LinearCombination::singleton((0, matching)),
            source: *on_this,
            target: *on_this,
            is_def_tl: true,
        }
    }
}

impl<T> Composable<usize> for BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy + AddAssign + Mul<Output = T> + MulAssign,
{
    fn compose(&self, other: &Self) -> Result<Self, String> {
        /*
        put the domain and codomain information into each term to get ExtendedPerfectMatching
        the multiplication implementation on ExtendedPerfectMatching
        which describes the composition of single terms
        so that induces the implementation of multiplication on LinearCombination<T,ExtendedPerfectMatching>
        then put that information back into a BrauerMorphism<T>
        */
        self.composable(other)?;
        let extended_diagram_self = self.diagram.inj_linearly_extend(|(delta_pow, diagram)| {
            ExtendedPerfectMatching((self.domain(), self.codomain(), delta_pow, diagram))
        });
        let extended_diagram_other = other.diagram.inj_linearly_extend(|(delta_pow, diagram)| {
            ExtendedPerfectMatching((other.domain(), other.codomain(), delta_pow, diagram))
        });
        let extended_diagram_product = extended_diagram_self * extended_diagram_other;
        let diagram =
            extended_diagram_product.linearly_extend(|extended| (extended.0 .2, extended.0 .3));
        Ok(Self {
            diagram,
            source: self.domain(),
            target: other.codomain(),
            is_def_tl: self.is_def_tl && other.is_def_tl,
        })
    }

    fn domain(&self) -> usize {
        self.source
    }

    fn codomain(&self) -> usize {
        self.target
    }
}

impl<T> Monoidal for BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy + AddAssign + Mul<Output = T> + MulAssign,
{
    fn monoidal(&mut self, other: Self) {
        let old_domain = self.domain();
        let old_codomain = self.codomain();
        let other_domain = other.domain();
        self.source += other_domain;
        self.target += other.codomain();
        let new_domain = self.domain();
        self.is_def_tl &= other.is_def_tl;
        self.diagram = self.diagram.linear_combine(
            other.diagram,
            |(delta_pow1, matching_1), (delta_pow2, matching2)| {
                let mut new_matching = matching_1.shift_index(old_domain, other_domain);
                let mut other_shifted = matching2.shift_index(0, old_domain);
                other_shifted = other_shifted.shift_index(new_domain, old_codomain);
                new_matching.pairs.extend(other_shifted.pairs);
                new_matching.canonicalize();
                (delta_pow1 + delta_pow2, new_matching)
            },
        );
    }
}

impl<T> MonoidalMorphism<usize> for BrauerMorphism<T> where
    T: Add<Output = T> + Zero + One + Copy + AddAssign + Mul<Output = T> + MulAssign
{
}

impl<T> BrauerMorphism<T>
where
    T: Add<Output = T> + Zero + One + Copy + AddAssign + Mul<Output = T> + MulAssign,
{
    #[allow(dead_code)]
    pub fn temperley_lieb_gens(n: usize) -> Vec<Self> {
        /*
        give the Temperley-Lieb e_1 \cdots e_{n-1}
        as elements of Hom_{Brauer}(n,n)
        */
        (0..n - 1)
            .map(|i| Self {
                diagram: LinearCombination::singleton((
                    0,
                    (0..n)
                        .map(|j| {
                            (if j == i {
                                (i, i + 1)
                            } else if j == i + 1 {
                                (i + n, i + 1 + n)
                            } else {
                                (j, j + n)
                            })
                            .into()
                        })
                        .collect(),
                )),
                source: n,
                target: n,
                is_def_tl: true,
            })
            .collect()
    }

    #[allow(dead_code)]
    pub fn symmetric_alg_gens(n: usize) -> Vec<Self> {
        /*
        give the symmetric algebra s_1 \cdots s_{n-1}
        where s_i permutes i and i+1 in 1..n (interpreted in domain and codomain appropriately)
        matches the rest with their shift in the codomain
        as elements of Hom_{Brauer}(n,n)
        */
        (0..(n - 1))
            .map(|i| Self {
                diagram: LinearCombination::singleton((
                    0,
                    (0..n)
                        .map(|j| {
                            (if j == i {
                                (i, i + n + 1)
                            } else if j == i + 1 {
                                (i + 1, i + n)
                            } else {
                                (j, j + n)
                            })
                            .into()
                        })
                        .collect(),
                )),
                source: n,
                target: n,
                is_def_tl: false,
            })
            .collect()
    }

    pub fn delta_polynomial(coeffs: &[T]) -> Self {
        /*
        The morphisms in Hom_{Brauer}(0,0) are in the polynomial ring T[delta]
        Give such an element upon specifying the coefficients of such a polynomial
        */
        let zeroth_coeff = *coeffs.first().unwrap_or(&T::zero());
        let empty_matching = PerfectMatching { pairs: vec![] };
        let mut diagram = LinearCombination::singleton((0, empty_matching));
        diagram *= zeroth_coeff;
        for (idx, cur_coeff) in coeffs.iter().enumerate().skip(1) {
            let empty_matching = PerfectMatching { pairs: vec![] };
            let mut cur_diagram = LinearCombination::singleton((idx, empty_matching));
            cur_diagram *= *cur_coeff;
            diagram += cur_diagram;
        }
        Self {
            diagram,
            source: 0,
            target: 0,
            is_def_tl: true,
        }
    }

    #[allow(dead_code)]
    pub fn dagger<F>(&self, num_dagger: F) -> Self
    where
        F: Fn(T) -> T,
    {
        /*
        for each term, flip the diagram upside down and change the coefficient to it's daggger
        as specified by the num_dagger function
        */
        let mut diagram = self
            .diagram
            .inj_linearly_extend(|(d, m)| (d, m.flip_upside_down(self.source, self.target)));
        diagram.change_coeffs(num_dagger);
        Self {
            diagram,
            source: self.target,
            target: self.source,
            is_def_tl: self.is_def_tl,
        }
    }

    #[allow(dead_code)]
    pub fn set_is_tl(&mut self) {
        /*
        if not sure that it is definitely a Temperley-Lieb morphism,
        then check the diagrams in all the terms
        if already sure, then don't need to check
        */
        if self.is_def_tl {
            return;
        }
        self.is_def_tl = self
            .diagram
            .all_terms_satisfy(|(_, p)| p.non_crossing(self.source, self.target));
    }
}

fn simplify<T>(me: &mut BrauerMorphism<T>)
where
    T: Add<Output = T> + Zero + One + Copy + AddAssign + Mul<Output = T> + MulAssign + Eq,
{
    /*
    get rid of all the terms with zero coefficient
    */
    me.diagram.simplify();
}

mod test {
    use std::ops::{AddAssign, MulAssign};

    use super::BrauerMorphism;
    use either::Either;
    use num::{One, Zero};

    #[allow(dead_code)]
    fn test_helper<T: Eq + AddAssign + MulAssign + Copy + One + Zero>(
        e_i: &[BrauerMorphism<T>],
        s_i: &[BrauerMorphism<T>],
        prod_these: &[Either<usize, usize>],
        delta_poly_coeffs: &[T],
    ) -> Result<BrauerMorphism<T>, String> {
        fn get_generator<T: Clone>(l_gens: &[T], r_gens: &[T], which: Either<usize, usize>) -> T {
            use crate::utils::EitherExt;
            which.join(|n| l_gens[n].clone(), |n| r_gens[n].clone())
        }
        use super::simplify;
        use crate::{category::Composable, monoidal::Monoidal};
        assert!(!prod_these.is_empty());
        let prod_these_0 = get_generator(e_i, s_i, prod_these[0]);
        let mut delta_poly = BrauerMorphism::delta_polynomial(delta_poly_coeffs);
        simplify(&mut delta_poly);
        if prod_these.len() == 1 {
            let mut full_prod = prod_these_0;
            full_prod.monoidal(delta_poly);
            return Ok(full_prod);
        }
        let prod_these_1 = get_generator(e_i, s_i, prod_these[1]);
        let mut full_prod = prod_these_0.compose(&prod_these_1);
        for cur_idx in prod_these.iter().skip(2) {
            let cur = get_generator(e_i, s_i, *cur_idx);
            full_prod = full_prod.and_then(|z| z.compose(&cur));
        }
        match full_prod {
            Ok(mut t) => {
                t.monoidal(delta_poly);
                Ok(t)
            }
            Err(e) => Err(e),
        }
    }

    #[test]
    fn t_l_relations() {
        use crate::{category::Composable, utils::test_asserter};
        use either::Either::Left;
        use num::Complex;
        let e_i = BrauerMorphism::<Complex<i32>>::temperley_lieb_gens(5);
        let delta_coeffs: [Complex<i32>; 2] = [<_>::zero(), <_>::one()];
        for idx in 0..e_i.len() {
            assert!(e_i[idx].is_def_tl);
            let e_i_dag = e_i[idx].dagger(|z| z.conj());
            assert!(
                &e_i[idx] == &e_i_dag,
                "{:?} vs {:?} when checking self adjointness of e_i",
                e_i[idx],
                e_i_dag
            );
            let e_ie_i = e_i[idx].compose(&e_i[idx]);
            let deltae_i = test_helper(&e_i, &[], &[Left(idx)], &delta_coeffs);
            test_asserter(
                e_ie_i,
                deltae_i,
                |j, k| j.is_def_tl && k.is_def_tl,
                "e_i e_i = delta e_i",
            );
            if idx < e_i.len() - 1 {
                let prod_iji = e_i[idx]
                    .compose(&e_i[idx + 1])
                    .and_then(|z| z.compose(&e_i[idx]));
                test_asserter(
                    prod_iji,
                    Ok(e_i[idx].clone()),
                    |j, k| j.is_def_tl && k.is_def_tl,
                    "e_i e_(i+1) e_i = e_i",
                );
            }
            if idx > 1 {
                let prod_iji = e_i[idx]
                    .compose(&e_i[idx - 1])
                    .and_then(|z| z.compose(&e_i[idx]));
                test_asserter(
                    prod_iji,
                    Ok(e_i[idx].clone()),
                    |j, k| j.is_def_tl && k.is_def_tl,
                    "e_i e_(i-1) e_i = e_i",
                );
            }
            for jdx in idx + 2..e_i.len() {
                let prod_ij = e_i[idx].compose(&e_i[jdx]);
                let prod_ji = e_i[jdx].compose(&e_i[idx]);
                test_asserter(
                    prod_ij,
                    prod_ji,
                    |j, k| j.is_def_tl && k.is_def_tl,
                    "e_i e_j = e_j e_i",
                );
            }
        }
    }

    #[test]
    fn wiki_example() {
        use super::{simplify, BrauerMorphism};
        use crate::{category::Composable, monoidal::Monoidal};
        use num::Complex;
        let e_i = BrauerMorphism::<Complex<i32>>::temperley_lieb_gens(5);
        let zero_complex = Complex::<i32>::zero();
        let one_complex = Complex::<i32>::one();
        let prod_1432 = e_i[0]
            .compose(&e_i[3])
            .and_then(|z| z.compose(&e_i[2]))
            .and_then(|z| z.compose(&e_i[1]));
        let prod_243 = e_i[1].compose(&e_i[3]).and_then(|z| z.compose(&e_i[2]));
        let prod_143243 = e_i[0]
            .compose(&e_i[3])
            .and_then(|z| z.compose(&e_i[2]))
            .and_then(|z| z.compose(&e_i[1]))
            .and_then(|z| z.compose(&e_i[3]))
            .and_then(|z| z.compose(&e_i[2]));
        let observed = prod_1432.and_then(|z| match prod_243 {
            Ok(real_prod_243) => z.compose(&real_prod_243),
            Err(e) => Err(e),
        });
        let mut expected =
            BrauerMorphism::<Complex<i32>>::delta_polynomial(&[zero_complex, one_complex]);
        simplify(&mut expected);

        match (observed, prod_143243) {
            (Ok(real_obs), Ok(exp_wo_delta)) => {
                assert!(real_obs.is_def_tl);
                expected.monoidal(exp_wo_delta);
                assert!(expected.is_def_tl);
                assert!(PartialEq::eq(&real_obs, &expected));
            }
            _ => {
                panic!("Error in composition when checking (e_1 e_4 e_3 e_2) (e_2 e_4 e_3) = delta e_1 e_4 e_3 e_2 e_4 e_3")
            }
        }
    }

    #[test]
    fn sym_relations() {
        use super::BrauerMorphism;
        use crate::{
            category::{Composable, HasIdentity},
            utils::test_asserter,
        };
        use either::Either::Right;
        use num::Complex;
        let n = 7;
        let s_i = BrauerMorphism::<Complex<i32>>::symmetric_alg_gens(n);
        let one_poly_coeffs = [Complex::<i32>::one()];
        let identity = BrauerMorphism::<Complex<i32>>::identity(&n);
        for idx in 0..n - 1 {
            assert!(!s_i[idx].is_def_tl);
            let s_i_dag = s_i[idx].dagger(|z| z.conj());
            assert!(
                PartialEq::eq(&s_i[idx], &s_i_dag),
                "{:?} vs {:?} when checking self adjointness of s_i",
                s_i[idx],
                s_i_dag
            );
            let s_is_i = s_i[idx].compose(&s_i[idx]);
            test_asserter(
                s_is_i,
                Ok(identity.clone()),
                |j, k| !j.is_def_tl && k.is_def_tl,
                "s_i s_i = 1",
            );
            if idx < n - 2 {
                let s_is_js_i = test_helper(
                    &[],
                    &s_i,
                    &[Right(idx), Right(idx + 1), Right(idx)],
                    &one_poly_coeffs,
                );
                let s_js_is_j = test_helper(
                    &[],
                    &s_i,
                    &[Right(idx + 1), Right(idx), Right(idx + 1)],
                    &one_poly_coeffs,
                );
                test_asserter(
                    s_is_js_i,
                    s_js_is_j,
                    |j, k| !j.is_def_tl && !k.is_def_tl,
                    "s_i s_(i+1) s_i = s_(i+1) s_i s_(i+1)",
                );
            }
            if idx > 1 {
                let s_is_js_i = test_helper(
                    &[],
                    &s_i,
                    &[Right(idx), Right(idx - 1), Right(idx)],
                    &one_poly_coeffs,
                );
                let s_js_is_j = test_helper(
                    &[],
                    &s_i,
                    &[Right(idx - 1), Right(idx), Right(idx - 1)],
                    &one_poly_coeffs,
                );
                test_asserter(
                    s_is_js_i,
                    s_js_is_j,
                    |j, k| !j.is_def_tl && !k.is_def_tl,
                    "s_i s_(i-1) s_i = s_(i-1) s_i s_(i-1)",
                );
            }
            for jdx in idx + 2..s_i.len() {
                let prod_ij = s_i[idx].compose(&s_i[jdx]);
                let prod_ji = s_i[jdx].compose(&s_i[idx]);
                test_asserter(
                    prod_ij,
                    prod_ji,
                    |j, k| !j.is_def_tl && !k.is_def_tl,
                    "s_i s_j = s_j s_i",
                );
            }
        }
    }

    #[test]
    fn tangle_relations() {
        use super::BrauerMorphism;
        use crate::{category::Composable, utils::test_asserter};
        use either::Either::{Left, Right};
        use num::Complex;
        let n = 7;
        let s_i = BrauerMorphism::<Complex<i32>>::symmetric_alg_gens(n);
        let e_i = BrauerMorphism::<Complex<i32>>::temperley_lieb_gens(n);
        let one_poly_coeffs = [Complex::<i32>::one()];
        for idx in 0..n - 1 {
            let e_is_i = e_i[idx].compose(&s_i[idx]);
            let s_ie_i: Result<BrauerMorphism<Complex<i32>>, String> = s_i[idx].compose(&e_i[idx]);
            test_asserter(
                e_is_i,
                Ok(e_i[idx].clone()),
                |j, k| !j.is_def_tl && k.is_def_tl,
                "e_i s_i = e_i",
            );
            test_asserter(
                s_ie_i,
                Ok(e_i[idx].clone()),
                |j, k| !j.is_def_tl && k.is_def_tl,
                "s_i e_i = e_i",
            );
            if idx < n - 2 {
                let s_is_je_i = test_helper(
                    &e_i,
                    &s_i,
                    &[Right(idx), Right(idx + 1), Left(idx)],
                    &one_poly_coeffs,
                );
                let e_je_i = test_helper(&e_i, &s_i, &[Left(idx + 1), Left(idx)], &one_poly_coeffs);
                test_asserter(
                    s_is_je_i,
                    e_je_i,
                    |j, k| !j.is_def_tl && k.is_def_tl,
                    "s_i s_(i+1) e_i = e_(i+1) e_i",
                );
                let e_is_je_i = test_helper(
                    &e_i,
                    &s_i,
                    &[Left(idx), Right(idx + 1), Left(idx)],
                    &one_poly_coeffs,
                );
                test_asserter(
                    e_is_je_i,
                    Ok(e_i[idx].clone()),
                    |j, k| !j.is_def_tl && k.is_def_tl,
                    "e_i s_(i+1) e_i = e_i",
                );
            }
            if idx > 1 {
                let s_is_je_i = test_helper(
                    &e_i,
                    &s_i,
                    &[Right(idx), Right(idx - 1), Left(idx)],
                    &one_poly_coeffs,
                );
                let e_je_i = test_helper(&e_i, &s_i, &[Left(idx - 1), Left(idx)], &one_poly_coeffs);
                test_asserter(
                    s_is_je_i,
                    e_je_i,
                    |j, k| !j.is_def_tl && k.is_def_tl,
                    "s_i s_(i-1) e_i = e_(i-1) e_i",
                );
                let e_is_je_i = test_helper(
                    &e_i,
                    &s_i,
                    &[Left(idx), Right(idx - 1), Left(idx)],
                    &one_poly_coeffs,
                );
                test_asserter(
                    e_is_je_i,
                    Ok(e_i[idx].clone()),
                    |j, k| !j.is_def_tl && k.is_def_tl,
                    "e_i s_(i-1) e_i = e_i",
                );
            }
            for jdx in idx + 2..s_i.len() {
                let prod_ij = s_i[idx].compose(&e_i[jdx]);
                let prod_ji = e_i[jdx].compose(&s_i[idx]);
                test_asserter(
                    prod_ij,
                    prod_ji,
                    |j, k| !j.is_def_tl && !k.is_def_tl,
                    "s_i e_j = e_j s_i",
                );
            }
        }
    }
}
